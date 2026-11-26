//! Background work: git operations run on a worker thread so the UI never
//! blocks, with results and conflict prompts delivered back over channels.
//!
//! The sync conflict resolver mirrors the phones' `BlockingResolver` / iOS
//! `UIResolver`: pass-core calls it once per conflicted path on the worker
//! thread, and it blocks that thread until the UI sends a choice back. Here the
//! block is a channel `recv()`; on the phones it is a CompletableDeferred /
//! DispatchSemaphore. Same contract.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::sync::{self, ConflictChoice, SyncOutcome};

/// A one-shot background op (clone/publish/fetch/push). Poll [`Job::poll`] each
/// frame; it yields the human-readable result once, then nothing.
pub struct Job {
    rx: Receiver<Result<String, String>>,
    _handle: Option<JoinHandle<()>>,
}

impl Job {
    pub fn poll(&self) -> Option<Result<String, String>> {
        self.rx.try_recv().ok()
    }
}

/// Spawn a background op returning a status string.
pub fn spawn_op<F>(f: F) -> Job
where
    F: FnOnce() -> anyhow::Result<String> + Send + 'static,
{
    let (tx, rx) = channel();
    let handle = thread::spawn(move || {
        let _ = tx.send(f().map_err(|e| e.to_string()));
    });
    Job {
        rx,
        _handle: Some(handle),
    }
}

/// What a running sync sends back to the UI.
pub enum SyncEvent {
    /// A conflicted path needs a decision; the worker is blocked until
    /// [`SyncJob::resolve`] answers.
    Conflict(String),
    /// The sync finished (Ok summary, or Err message).
    Done(Result<String, String>),
}

/// A running sync, with its conflict round-trip channel.
pub struct SyncJob {
    events: Receiver<SyncEvent>,
    replies: Sender<ConflictChoice>,
    _handle: Option<JoinHandle<()>>,
}

impl SyncJob {
    pub fn poll(&self) -> Option<SyncEvent> {
        self.events.try_recv().ok()
    }

    /// Answer the outstanding conflict, unblocking the worker.
    pub fn resolve(&self, choice: ConflictChoice) {
        let _ = self.replies.send(choice);
    }
}

/// Spawn a sync on the store at `root`. Conflicts surface as
/// [`SyncEvent::Conflict`]; answer each with [`SyncJob::resolve`].
pub fn spawn_sync(root: PathBuf) -> SyncJob {
    let (ev_tx, ev_rx) = channel::<SyncEvent>();
    let (rep_tx, rep_rx) = channel::<ConflictChoice>();
    let handle = thread::spawn(move || {
        let mut resolver = |path: &str| -> ConflictChoice {
            let _ = ev_tx.send(SyncEvent::Conflict(path.to_string()));
            // Block until the UI answers; default to the safe choice if the UI
            // went away (channel closed).
            rep_rx.recv().unwrap_or(ConflictChoice::KeepBoth)
        };
        let result = sync::sync(&root, &mut resolver)
            .map(summarize)
            .map_err(|e| e.to_string());
        let _ = ev_tx.send(SyncEvent::Done(result));
    });
    SyncJob {
        events: ev_rx,
        replies: rep_tx,
        _handle: Some(handle),
    }
}

/// A one-line summary of a sync outcome for the status bar.
#[must_use]
pub fn summarize(outcome: SyncOutcome) -> String {
    match outcome {
        SyncOutcome::UpToDate => "Already up to date.".into(),
        SyncOutcome::FastForwarded => "Updated from the remote.".into(),
        SyncOutcome::Rebased { replayed } => {
            format!("Synced; replayed {replayed} local commit(s).")
        }
        SyncOutcome::ResolvedConflicts {
            resolved,
            kept_both,
        } => format!(
            "Synced; resolved {} conflict(s){}.",
            resolved.len(),
            if kept_both.is_empty() {
                String::new()
            } else {
                format!(", kept both on {}", kept_both.join(", "))
            }
        ),
    }
}

/// What a YubiKey decrypt sends back to the UI.
pub enum DecryptEvent {
    /// Something the plugin wants shown ("touch your YubiKey").
    Message(String),
    /// A yes/no question; the worker waits for [`DecryptJob::confirm`].
    Confirm {
        text: String,
        yes: String,
        no: Option<String>,
    },
    /// A secret (the PIV PIN); the worker waits for [`DecryptJob::secret`].
    Secret(String),
    /// The decrypt finished.
    Done(Result<pass_core::entry::Entry, String>),
}

/// An answer to a [`DecryptEvent::Confirm`] or [`DecryptEvent::Secret`].
enum PromptReply {
    Confirm(Option<bool>),
    Secret(Option<age::secrecy::SecretString>),
}

/// A passage entry being opened on a worker thread, because opening it may
/// wait on a YubiKey: its plugin's prompts come back as [`DecryptEvent`]s.
pub struct DecryptJob {
    /// The entry being opened.
    pub name: String,
    events: Receiver<DecryptEvent>,
    replies: Sender<PromptReply>,
    _handle: Option<JoinHandle<()>>,
}

impl DecryptJob {
    pub fn poll(&self) -> Option<DecryptEvent> {
        self.events.try_recv().ok()
    }

    /// Answer the outstanding yes/no question (`None`: dismissed).
    pub fn confirm(&self, answer: Option<bool>) {
        let _ = self.replies.send(PromptReply::Confirm(answer));
    }

    /// Answer the outstanding secret request (`None`: cancelled).
    pub fn secret(&self, answer: Option<age::secrecy::SecretString>) {
        let _ = self.replies.send(PromptReply::Secret(answer));
    }
}

/// Forwards the plugin's prompts to the UI and blocks for the answers. If the
/// UI has gone (the job was dropped), every prompt is a cancel.
struct ChannelPrompter {
    events: Sender<DecryptEvent>,
    replies: std::sync::Mutex<Receiver<PromptReply>>,
}

impl ChannelPrompter {
    fn ask(&self, event: DecryptEvent) -> Option<PromptReply> {
        self.events.send(event).ok()?;
        self.replies.lock().ok()?.recv().ok()
    }
}

impl crate::crypto::age_backend::PluginPrompter for ChannelPrompter {
    fn message(&self, text: &str) {
        let _ = self.events.send(DecryptEvent::Message(text.to_owned()));
    }

    fn confirm(&self, text: &str, yes: &str, no: Option<&str>) -> Option<bool> {
        match self.ask(DecryptEvent::Confirm {
            text: text.to_owned(),
            yes: yes.to_owned(),
            no: no.map(str::to_owned),
        })? {
            PromptReply::Confirm(answer) => answer,
            PromptReply::Secret(_) => None,
        }
    }

    fn secret(&self, description: &str) -> Option<age::secrecy::SecretString> {
        match self.ask(DecryptEvent::Secret(description.to_owned()))? {
            PromptReply::Secret(answer) => answer,
            PromptReply::Confirm(_) => None,
        }
    }
}

/// Open passage entry `name` of the store at `root` on a worker thread with
/// `backend`, whose plugin prompts come back as [`DecryptEvent`]s.
pub fn spawn_decrypt(
    root: PathBuf,
    name: String,
    backend: crate::crypto::AgeBackend,
) -> DecryptJob {
    let (ev_tx, ev_rx) = channel::<DecryptEvent>();
    let (rep_tx, rep_rx) = channel::<PromptReply>();
    let entry_name = name.clone();
    let handle = thread::spawn(move || {
        let prompter = std::sync::Arc::new(ChannelPrompter {
            events: ev_tx.clone(),
            replies: std::sync::Mutex::new(rep_rx),
        });
        let backend = backend.with_prompter(prompter);
        let result = pass_core::store::Store::open(root, pass_core::store::StoreFormat::Passage)
            .map_err(|e| e.to_string())
            .and_then(|store| {
                store
                    .read_entry(
                        &entry_name,
                        &backend as &dyn pass_core::crypto::CryptoBackend,
                    )
                    .map_err(|e| e.to_string())
            });
        let _ = ev_tx.send(DecryptEvent::Done(result));
    });
    DecryptJob {
        name,
        events: ev_rx,
        replies: rep_tx,
        _handle: Some(handle),
    }
}
