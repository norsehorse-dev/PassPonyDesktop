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
