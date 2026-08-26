//! The App struct: all persistent UI state, plus the eframe entry point.
//!
//! The shell is a fixed left rail of four destinations (Passwords, Keys, Sync,
//! Settings) over a content area that shows the selected screen — the layout
//! from the approved mockup, ported from AgePony Desktop. `logic` runs the
//! per-frame state work (jobs, timers, shortcuts, drops) and `ui` paints the
//! rail and the current screen. `save` remembers the screen, format, and the
//! last store opened for each format, so a relaunch lands where you left off.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use pass_core::crypto::CryptoBackend;
use pass_core::entry::Entry;
use pass_core::store::{Store, StoreFormat};

use crate::entry_view::EntryView;
use crate::i18n::{self, Lang};
use crate::sync::{ConflictChoice, SyncStatus};
use crate::theme::{self, ThemeChoice};
use crate::{edit, keys, panels, sync, tasks};

/// The four rail destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Passwords,
    Keys,
    Sync,
    Settings,
}

impl Screen {
    pub const ALL: [Screen; 4] = [
        Screen::Passwords,
        Screen::Keys,
        Screen::Sync,
        Screen::Settings,
    ];

    fn code(self) -> u8 {
        match self {
            Screen::Passwords => 0,
            Screen::Keys => 1,
            Screen::Sync => 2,
            Screen::Settings => 3,
        }
    }

    fn from_code(c: u8) -> Screen {
        match c {
            1 => Screen::Keys,
            2 => Screen::Sync,
            3 => Screen::Settings,
            _ => Screen::Passwords,
        }
    }
}

/// What `save` persists between runs (window geometry is handled by eframe).
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Prefs {
    screen: u8,
    format: u8,
    pass_store: Option<String>,
    passage_store: Option<String>,
}

/// Storage key for [`Prefs`].
const PREFS_KEY: &str = "passpony_prefs";

/// The open editor's buffer. `text`'s first line is the password; the rest are
/// fields and any `otpauth://` line, exactly as `pass edit` treats the file.
pub struct EditState {
    pub name: String,
    pub is_new: bool,
    pub text: String,
    pub gen_len: usize,
}

/// Steps of the first-run tour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnbStep {
    Welcome,
    Keys,
    Backup,
    Store,
}

/// First-run wizard state.
pub struct Onboarding {
    pub step: OnbStep,
    pub format: StoreFormat,
    pub recipient: Option<String>,
    /// True once an age identity exists (freshly generated or already on disk).
    pub identity_ready: bool,
    pub backed_up: bool,
    pub store_path: String,
}

impl Onboarding {
    fn new() -> Self {
        Self {
            step: OnbStep::Welcome,
            format: StoreFormat::Pass,
            recipient: None,
            identity_ready: false,
            backed_up: false,
            store_path: String::new(),
        }
    }
}

fn commit_status(action: &str, name: &str, c: &edit::Commit) -> String {
    match c {
        edit::Commit::Committed => format!("{action} {name}."),
        edit::Commit::NotARepo => format!("{action} {name} (saved; not a git repo)."),
        edit::Commit::Failed(e) => format!("{action} {name} (saved; commit failed: {e})"),
    }
}

/// How long a typed pass passphrase stays cached in memory. The phones use five
/// minutes; a later phase makes it a setting.
const PASSPHRASE_TTL: Duration = Duration::from_secs(5 * 60);
/// How long a copied secret lives on the clipboard before it is cleared,
/// matching the phones.
pub const CLIPBOARD_TTL: Duration = Duration::from_secs(45);

pub struct App {
    /// The rail destination currently shown.
    pub screen: Screen,
    /// The rail-head artwork texture, uploaded once on first paint.
    rail_tex: Option<egui::TextureHandle>,
    /// Set by ⌘F; consumed by the Passwords screen to focus the search box.
    pub focus_search: bool,

    pub format: StoreFormat,
    pub store: Option<Store>,
    pub entries: Option<Vec<String>>,
    pub selected: Option<String>,
    pub search: String,
    pub status: String,

    /// The last store opened for each format, remembered across runs.
    pub pass_store_path: Option<PathBuf>,
    pub passage_store_path: Option<PathBuf>,

    // Keys / settings.
    pub key_files: Vec<String>,
    pub passphrase: String,
    passphrase_set_at: Option<Instant>,

    // The opened entry.
    pub entry: Option<EntryView>,
    pub entry_error: Option<String>,
    pub reveal: bool,

    // Clipboard with a delayed clear.
    clipboard: Option<arboard::Clipboard>,
    clip_pending: Option<(String, Instant)>,

    // Editing.
    pub editing: Option<EditState>,
    pub confirm_delete: bool,

    // Sync.
    pub sync_status: Option<SyncStatus>,
    pub remote_display: Option<String>,
    pub sync_url: String,
    pub sync_user: String,
    pub sync_token: String,
    pub pending_conflict: Option<String>,
    pub busy: bool,
    op_job: Option<tasks::Job>,
    sync_job: Option<tasks::SyncJob>,
    pending_clone_dest: Option<PathBuf>,

    // First-run onboarding.
    pub onboarding: Option<Onboarding>,

    // Settings / preferences.
    pub lang: Lang,
    pub theme: ThemeChoice,
    pub confirm_panic: bool,
}

impl App {
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let lang = keys::load_language()
            .and_then(|c| Lang::from_code(&c))
            .unwrap_or(Lang::En);
        i18n::set(lang);
        crate::theme::install_fonts(&cc.egui_ctx);
        let theme = keys::load_theme()
            .map(|c| ThemeChoice::from_code(&c))
            .unwrap_or(ThemeChoice::Auto);
        theme.apply(&cc.egui_ctx);

        // Restore last screen, format, and remembered store paths.
        let prefs: Prefs = cc
            .storage
            .and_then(|s| eframe::get_value(s, PREFS_KEY))
            .unwrap_or_default();
        let format = if prefs.format == 1 {
            StoreFormat::Passage
        } else {
            StoreFormat::Pass
        };

        let mut app = Self {
            screen: Screen::from_code(prefs.screen),
            rail_tex: None,
            focus_search: false,
            format,
            store: None,
            entries: None,
            selected: None,
            search: String::new(),
            status: String::new(),
            pass_store_path: prefs.pass_store.map(PathBuf::from),
            passage_store_path: prefs.passage_store.map(PathBuf::from),
            key_files: keys::list_pgp_key_files().unwrap_or_default(),
            passphrase: String::new(),
            passphrase_set_at: None,
            entry: None,
            entry_error: None,
            reveal: false,
            clipboard: arboard::Clipboard::new().ok(),
            clip_pending: None,
            editing: None,
            confirm_delete: false,
            sync_status: None,
            remote_display: None,
            sync_url: String::new(),
            sync_user: String::new(),
            sync_token: String::new(),
            pending_conflict: None,
            busy: false,
            op_job: None,
            sync_job: None,
            pending_clone_dest: None,
            onboarding: (!keys::is_onboarded()).then(Onboarding::new),
            lang,
            theme,
            confirm_panic: false,
        };
        // If onboarding is done, open a store so a launch lands in the browser
        // rather than an empty screen: the one we remember for this format, or
        // the platform default location if it exists.
        if app.onboarding.is_none() {
            app.open_best_store();
        }
        app
    }

    /// The remembered store path for a format, if any.
    #[must_use]
    fn remembered_path(&self, format: StoreFormat) -> Option<PathBuf> {
        match format {
            StoreFormat::Pass => self.pass_store_path.clone(),
            StoreFormat::Passage => self.passage_store_path.clone(),
        }
    }

    /// The default store location for a format under the user's home.
    #[must_use]
    fn default_store_path(&self, format: StoreFormat) -> Option<PathBuf> {
        directories::UserDirs::new().map(|d| d.home_dir().join(format.default_store_dir()))
    }

    /// Open the best store for the current format: the remembered one if it
    /// still exists, else the platform default location if it exists. Silent
    /// when neither is present (the Passwords screen shows its empty state).
    fn open_best_store(&mut self) {
        if let Some(path) = self.remembered_path(self.format) {
            if path.exists() {
                self.open_store_at(path);
                return;
            }
        }
        if let Some(path) = self.default_store_path(self.format) {
            if path.exists() {
                self.open_store_at(path);
            }
        }
    }

    fn prefs(&self) -> Prefs {
        Prefs {
            screen: self.screen.code(),
            format: match self.format {
                StoreFormat::Pass => 0,
                StoreFormat::Passage => 1,
            },
            pass_store: self
                .pass_store_path
                .as_ref()
                .map(|p| p.display().to_string()),
            passage_store: self
                .passage_store_path
                .as_ref()
                .map(|p| p.display().to_string()),
        }
    }

    /// Switch stores when the format changes: reopen the remembered store for
    /// the newly-selected format, or its default location, the same as launch.
    pub fn set_format(&mut self, format: StoreFormat) {
        if format == self.format {
            return;
        }
        self.format = format;
        self.close_store();
        self.open_best_store();
    }

    pub fn set_language(&mut self, lang: Lang) {
        self.lang = lang;
        i18n::set(lang);
        keys::save_language(lang.code());
    }

    pub fn set_theme(&mut self, choice: ThemeChoice, ctx: &egui::Context) {
        self.theme = choice;
        choice.apply(ctx);
        keys::save_theme(choice.code());
    }

    /// Re-encrypt the whole store to its current recipients, on the worker.
    pub fn start_reencrypt(&mut self) {
        let Some(store) = &self.store else { return };
        let root = store.root().to_path_buf();
        let format = self.format;
        let pass = self.passphrase_opt();
        self.op_job = Some(tasks::spawn_op(move || {
            let store = Store::open(&root, format)?;
            let backend = keys::load_backend(format, pass.as_deref())?;
            let (n, _c) = edit::reencrypt(&store, "", backend.as_crypto())?;
            Ok(format!(
                "Re-encrypted {n} entr{}.",
                if n == 1 { "y" } else { "ies" }
            ))
        }));
        self.busy = true;
    }

    /// Export the age identity from Settings and mark it backed up.
    pub fn export_identity(&mut self) {
        let Some(dest) = rfd::FileDialog::new()
            .set_file_name("passpony-age-identity.txt")
            .save_file()
        else {
            return;
        };
        match keys::export_identities(&dest) {
            Ok(()) => {
                keys::mark_identity_backed_up();
                self.status = "Identity backed up. Keep it somewhere safe.".into();
            }
            Err(e) => self.status = format!("Backup failed: {e}"),
        }
    }

    pub fn run_panic_wipe(&mut self) {
        match keys::panic_wipe() {
            Ok(()) => {
                self.close_store();
                self.refresh_keys();
                self.status = "All keys and identities wiped.".into();
            }
            Err(e) => self.status = format!("Wipe failed: {e}"),
        }
        self.confirm_panic = false;
    }

    /// Open the config directory in the OS file manager.
    pub fn reveal_config_dir(&mut self) {
        if let Some(dir) = keys::config_dir() {
            let _ = std::fs::create_dir_all(&dir);
            let opener = if cfg!(target_os = "macos") {
                "open"
            } else if cfg!(target_os = "windows") {
                "explorer"
            } else {
                "xdg-open"
            };
            let _ = std::process::Command::new(opener).arg(&dir).spawn();
        }
    }

    fn home_store_path(&self, format: StoreFormat) -> String {
        directories::UserDirs::new()
            .map(|d| {
                d.home_dir()
                    .join(format.default_store_dir())
                    .display()
                    .to_string()
            })
            .unwrap_or_default()
    }

    /// Advance/retreat the wizard, filling the default store path when needed.
    pub fn onb_set_step(&mut self, step: OnbStep) {
        if step == OnbStep::Store {
            let need = self
                .onboarding
                .as_ref()
                .map(|o| o.store_path.is_empty())
                .unwrap_or(false);
            if need {
                let fmt = self
                    .onboarding
                    .as_ref()
                    .map(|o| o.format)
                    .unwrap_or(StoreFormat::Pass);
                let path = self.home_store_path(fmt);
                if let Some(onb) = &mut self.onboarding {
                    onb.store_path = path;
                }
            }
        }
        if let Some(onb) = &mut self.onboarding {
            onb.step = step;
        }
    }

    /// Generate the age identity for a new passage user (first-run).
    pub fn onb_generate_identity(&mut self) {
        match keys::ensure_age_identity() {
            Ok(made) => {
                if let Some(onb) = &mut self.onboarding {
                    // `made` is None when an identity already existed; either way
                    // one is now present, so the step is satisfied.
                    if made.is_some() {
                        onb.recipient = made;
                    }
                    onb.identity_ready = true;
                }
                self.status = "Age identity ready.".into();
            }
            Err(e) => self.status = format!("Could not create identity: {e}"),
        }
    }

    /// Import a pass key during onboarding and show its fingerprint.
    pub fn onb_import_key(&mut self) {
        self.import_pgp_dialog();
        let fpr = keys::first_pgp_fingerprint().ok().flatten();
        if let Some(onb) = &mut self.onboarding {
            onb.recipient = fpr;
        }
    }

    /// Export the age identity to a user-chosen file and mark it backed up.
    pub fn onb_export_identity(&mut self) {
        let Some(dest) = rfd::FileDialog::new()
            .set_file_name("passpony-age-identity.txt")
            .save_file()
        else {
            return;
        };
        match keys::export_identities(&dest) {
            Ok(()) => {
                keys::mark_identity_backed_up();
                if let Some(onb) = &mut self.onboarding {
                    onb.backed_up = true;
                }
                self.status = "Identity backed up. Keep it somewhere safe.".into();
            }
            Err(e) => self.status = format!("Backup failed: {e}"),
        }
    }

    /// Create the store the wizard configured and finish onboarding.
    pub fn onb_create_store(&mut self) {
        let Some(onb) = &self.onboarding else { return };
        let format = onb.format;
        let path = PathBuf::from(onb.store_path.trim());
        if path.as_os_str().is_empty() {
            self.status = "Choose where to create the store.".into();
            return;
        }
        let result = match format {
            StoreFormat::Pass => match keys::first_pgp_fingerprint() {
                Ok(Some(fpr)) => keys::init_pass_store(&path, &fpr),
                Ok(None) => Err(anyhow::anyhow!("import a pass key first")),
                Err(e) => Err(e),
            },
            StoreFormat::Passage => keys::init_passage_store(&path),
        };
        match result {
            Ok(()) => {
                keys::mark_onboarded();
                self.onboarding = None;
                self.format = format;
                self.open_store_at(path);
                self.status = "Store created.".into();
            }
            Err(e) => self.status = format!("Could not create store: {e}"),
        }
    }

    /// Finish onboarding to clone instead: mark done, jump to the Sync screen.
    pub fn onb_go_clone(&mut self) {
        keys::mark_onboarded();
        self.onboarding = None;
        self.screen = Screen::Sync;
    }

    pub fn onb_skip(&mut self) {
        keys::mark_onboarded();
        self.onboarding = None;
    }

    fn creds_url(&self, base: &str) -> String {
        sync::with_credentials(base, self.sync_user.trim(), self.sync_token.trim())
    }

    pub fn refresh_sync_status(&mut self) {
        let (st, rd) = match &self.store {
            Some(s) => (sync::status(s.root()), sync::redacted_remote(s.root())),
            None => (None, None),
        };
        self.sync_status = st;
        self.remote_display = rd;
    }

    pub fn start_publish(&mut self) {
        let Some(store) = &self.store else {
            self.status = "No store open.".into();
            return;
        };
        if self.sync_url.trim().is_empty() {
            self.status = "Enter a remote URL to publish to.".into();
            return;
        }
        let root = store.root().to_path_buf();
        let format = self.format;
        let url = self.creds_url(self.sync_url.trim());
        self.op_job = Some(tasks::spawn_op(move || {
            sync::publish(&root, format, &url).map(|()| "Published to remote.".to_string())
        }));
        self.busy = true;
    }

    pub fn start_fetch(&mut self) {
        let Some(store) = &self.store else { return };
        let root = store.root().to_path_buf();
        self.op_job = Some(tasks::spawn_op(move || {
            sync::fetch(&root).map(|()| "Fetched from remote.".to_string())
        }));
        self.busy = true;
    }

    pub fn start_push(&mut self) {
        let Some(store) = &self.store else { return };
        let root = store.root().to_path_buf();
        self.op_job = Some(tasks::spawn_op(move || {
            sync::push(&root).map(|()| "Pushed to remote.".to_string())
        }));
        self.busy = true;
    }

    pub fn start_sync(&mut self) {
        let Some(store) = &self.store else { return };
        let root = store.root().to_path_buf();
        self.sync_job = Some(tasks::spawn_sync(root));
        self.busy = true;
    }

    pub fn start_clone(&mut self) {
        if self.sync_url.trim().is_empty() {
            self.status = "Enter a remote URL to clone.".into();
            return;
        }
        let Some(dest) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        let url = self.creds_url(self.sync_url.trim());
        let dest2 = dest.clone();
        self.pending_clone_dest = Some(dest);
        self.op_job = Some(tasks::spawn_op(move || {
            sync::clone(&url, &dest2).map(|()| "Cloned.".to_string())
        }));
        self.busy = true;
    }

    pub fn resolve_conflict(&mut self, choice: ConflictChoice) {
        if let Some(job) = &self.sync_job {
            job.resolve(choice);
        }
        self.pending_conflict = None;
    }

    /// Drain worker channels. Called once per frame.
    pub fn poll_jobs(&mut self) {
        if let Some(job) = &self.op_job {
            if let Some(res) = job.poll() {
                self.op_job = None;
                self.busy = false;
                match res {
                    Ok(msg) => {
                        self.status = msg;
                        if let Some(dest) = self.pending_clone_dest.take() {
                            self.open_store_at(dest);
                        } else {
                            self.reload_entries();
                        }
                        self.refresh_sync_status();
                    }
                    Err(e) => {
                        self.pending_clone_dest = None;
                        self.status = format!("Failed: {e}");
                    }
                }
            }
        }

        // Collect sync events first, then apply, to avoid borrowing the job
        // while mutating self.
        let mut events = Vec::new();
        if let Some(job) = &self.sync_job {
            while let Some(ev) = job.poll() {
                events.push(ev);
            }
        }
        let mut done = false;
        for ev in events {
            match ev {
                tasks::SyncEvent::Conflict(path) => self.pending_conflict = Some(path),
                tasks::SyncEvent::Done(res) => {
                    done = true;
                    self.status = match res {
                        Ok(m) => m,
                        Err(e) => format!("Sync failed: {e}"),
                    };
                }
            }
        }
        if done {
            self.sync_job = None;
            self.busy = false;
            self.pending_conflict = None;
            self.reload_entries();
            self.refresh_sync_status();
        }
    }

    fn passphrase_opt(&self) -> Option<String> {
        if self.passphrase.is_empty() {
            None
        } else {
            Some(self.passphrase.clone())
        }
    }

    fn reload_entries(&mut self) {
        if let Some(store) = &self.store {
            if let Ok(es) = store.entries() {
                self.entries = Some(
                    es.into_iter()
                        .filter(|e| !e.hidden)
                        .map(|e| e.name)
                        .collect(),
                );
            }
        }
    }

    pub fn start_new_entry(&mut self) {
        self.confirm_delete = false;
        self.editing = Some(EditState {
            name: String::new(),
            is_new: true,
            text: String::new(),
            gen_len: 20,
        });
    }

    pub fn start_edit_selected(&mut self) {
        self.confirm_delete = false;
        if let (Some(name), Some(view)) = (self.selected.clone(), self.entry.as_ref()) {
            self.editing = Some(EditState {
                name,
                is_new: false,
                text: view.raw.to_string(),
                gen_len: 20,
            });
        }
    }

    pub fn cancel_edit(&mut self) {
        self.editing = None;
    }

    /// Save the open editor buffer: encrypt, write, and commit.
    pub fn save_edit(&mut self) {
        let Some(edit_state) = self.editing.take() else {
            return;
        };
        if edit_state.name.trim().is_empty() {
            self.status = "An entry name is required.".into();
            self.editing = Some(edit_state);
            return;
        }
        let backend = match keys::load_backend(self.format, self.passphrase_opt().as_deref()) {
            Ok(b) => b,
            Err(e) => {
                self.status = e.to_string();
                self.editing = Some(edit_state);
                return;
            }
        };
        if self.store.is_none() {
            self.status = "No store open.".into();
            self.editing = Some(edit_state);
            return;
        }
        let entry = Entry::from_bytes(edit_state.text.clone().into_bytes());
        let store = self.store.as_ref().expect("checked");
        let result = edit::save_entry(
            store,
            &edit_state.name,
            &entry,
            backend.as_crypto(),
            edit_state.is_new,
            false,
        );
        match result {
            Ok(outcome) => {
                self.status = commit_status("Saved", &edit_state.name, &outcome);
                let name = edit_state.name.clone();
                self.reload_entries();
                self.open_entry(name);
            }
            Err(e) => {
                self.status = e.to_string();
                self.editing = Some(edit_state);
            }
        }
    }

    pub fn delete_selected(&mut self) {
        let Some(name) = self.selected.clone() else {
            return;
        };
        if self.store.is_none() {
            self.status = "No store open.".into();
            return;
        }
        let store = self.store.as_ref().expect("checked");
        let result = edit::remove_entry(store, &name);
        match result {
            Ok(outcome) => {
                self.status = commit_status("Removed", &name, &outcome);
                self.selected = None;
                self.clear_entry_view();
                self.reload_entries();
            }
            Err(e) => self.status = e.to_string(),
        }
        self.confirm_delete = false;
    }

    pub fn close_store(&mut self) {
        self.store = None;
        self.entries = None;
        self.selected = None;
        self.editing = None;
        self.confirm_delete = false;
        self.clear_entry_view();
        self.status.clear();
    }

    fn clear_entry_view(&mut self) {
        self.entry = None;
        self.entry_error = None;
        self.reveal = false;
    }

    pub fn note_passphrase_changed(&mut self) {
        self.passphrase_set_at = Some(Instant::now());
    }

    /// Expire the cached passphrase once its TTL is up.
    pub fn expire_passphrase_if_stale(&mut self) {
        if let Some(at) = self.passphrase_set_at {
            if at.elapsed() >= PASSPHRASE_TTL {
                self.passphrase.clear();
                self.passphrase_set_at = None;
            }
        }
    }

    /// Copy a secret to the clipboard and schedule it to be cleared.
    pub fn copy_secret(&mut self, text: &str) {
        if let Some(cb) = &mut self.clipboard {
            if cb.set_text(text.to_owned()).is_ok() {
                self.clip_pending = Some((text.to_owned(), Instant::now()));
                self.status = format!("Copied. Clears in {}s.", CLIPBOARD_TTL.as_secs());
            } else {
                self.status = "Could not access the clipboard.".into();
            }
        } else {
            self.status = "No clipboard available.".into();
        }
    }

    /// Clear the clipboard once the TTL passes, but only if it still holds what
    /// we put there (never clobber something the user copied since).
    pub fn tick_clipboard(&mut self) {
        let Some((value, at)) = &self.clip_pending else {
            return;
        };
        if at.elapsed() < CLIPBOARD_TTL {
            return;
        }
        if let Some(cb) = &mut self.clipboard {
            let unchanged = cb.get_text().map(|c| &c == value).unwrap_or(false);
            if unchanged {
                let _ = cb.set_text(String::new());
            }
        }
        self.clip_pending = None;
    }

    /// Seconds until the pending clipboard clear, for the UI.
    #[must_use]
    pub fn clipboard_seconds_left(&self) -> Option<u64> {
        self.clip_pending
            .as_ref()
            .map(|(_, at)| CLIPBOARD_TTL.saturating_sub(at.elapsed()).as_secs())
    }

    pub fn refresh_keys(&mut self) {
        self.key_files = keys::list_pgp_key_files().unwrap_or_default();
    }

    pub fn import_pgp_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            self.import_pgp_path(&path);
        }
    }

    /// Import a PGP key from a known path (dialog or drag-and-drop).
    pub fn import_pgp_path(&mut self, path: &Path) {
        match keys::import_pgp_key(path) {
            Ok(dest) => {
                self.status = format!(
                    "Imported {}",
                    dest.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                );
                self.refresh_keys();
            }
            Err(e) => self.status = format!("Import failed: {e}"),
        }
    }

    /// Apply files dropped onto the window: a folder opens as a store; a key-ish
    /// file is imported as a PGP key. Anything else is reported, not guessed.
    fn handle_drops(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        for path in dropped {
            if path.is_dir() {
                self.open_store_at(path);
                self.screen = Screen::Passwords;
            } else if is_keyish(&path) {
                self.import_pgp_path(&path);
                self.screen = Screen::Keys;
            } else {
                self.status = format!(
                    "Not a key or store folder: {}",
                    path.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                );
            }
        }
    }

    /// Keyboard shortcuts: ⌘1–4 pick a screen, ⌘F focuses search, ⌘, opens
    /// Settings. (⌘ is Ctrl on Windows/Linux.)
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::Key;
        let (cmd, keys_down) = ctx.input(|i| {
            (
                i.modifiers.command,
                [
                    i.key_pressed(Key::Num1),
                    i.key_pressed(Key::Num2),
                    i.key_pressed(Key::Num3),
                    i.key_pressed(Key::Num4),
                    i.key_pressed(Key::F),
                    i.key_pressed(Key::Comma),
                ],
            )
        });
        if !cmd {
            return;
        }
        if keys_down[0] {
            self.screen = Screen::Passwords;
        } else if keys_down[1] {
            self.screen = Screen::Keys;
        } else if keys_down[2] {
            self.screen = Screen::Sync;
        } else if keys_down[3] {
            self.screen = Screen::Settings;
        } else if keys_down[4] {
            self.screen = Screen::Passwords;
            self.focus_search = true;
        } else if keys_down[5] {
            self.screen = Screen::Settings;
        }
    }

    pub fn generate_age_identity(&mut self) {
        match keys::generate_age_identity() {
            Ok(recipient) => self.status = format!("New age identity: {recipient}"),
            Err(e) => self.status = format!("Could not generate identity: {e}"),
        }
    }

    pub fn open_store_dialog(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            self.open_store_at(dir);
        }
    }

    pub fn open_default_store(&mut self) {
        match directories::UserDirs::new() {
            Some(dirs) => {
                let root = dirs.home_dir().join(self.format.default_store_dir());
                self.open_store_at(root);
            }
            None => self.status = "Cannot locate home directory.".into(),
        }
    }

    fn open_store_at(&mut self, root: PathBuf) {
        match Store::open(&root, self.format) {
            Ok(store) => match store.entries() {
                Ok(entries) => {
                    let names = entries
                        .into_iter()
                        .filter(|e| !e.hidden)
                        .map(|e| e.name)
                        .collect();
                    self.entries = Some(names);
                    self.store = Some(store);
                    self.selected = None;
                    self.clear_entry_view();
                    self.status.clear();
                    self.remember_store(&root);
                    self.refresh_sync_status();
                }
                Err(e) => {
                    self.status = format!("Opened, but could not list entries: {e}");
                    self.store = Some(store);
                    self.entries = Some(Vec::new());
                }
            },
            Err(e) => {
                self.status = format!("Cannot open {}: {e}", root.display());
                self.close_store();
            }
        }
    }

    /// Record a successfully-opened store as the remembered one for its format.
    fn remember_store(&mut self, root: &Path) {
        match self.format {
            StoreFormat::Pass => self.pass_store_path = Some(root.to_path_buf()),
            StoreFormat::Passage => self.passage_store_path = Some(root.to_path_buf()),
        }
    }

    /// Select an entry and decrypt it with the loaded keys/identities.
    pub fn open_entry(&mut self, name: String) {
        self.selected = Some(name.clone());
        self.editing = None;
        self.confirm_delete = false;
        self.clear_entry_view();
        let Some(store) = &self.store else {
            self.entry_error = Some("No store open.".into());
            return;
        };
        let passphrase = if self.passphrase.is_empty() {
            None
        } else {
            Some(self.passphrase.as_str())
        };
        let outcome = match self.format {
            StoreFormat::Pass => match keys::load_pgp_backend(passphrase) {
                Ok(b) => store.read_entry(&name, &b as &dyn CryptoBackend),
                Err(e) => {
                    self.entry_error = Some(e.to_string());
                    return;
                }
            },
            StoreFormat::Passage => match keys::load_age_backend() {
                Ok(b) => store.read_entry(&name, &b as &dyn CryptoBackend),
                Err(e) => {
                    self.entry_error = Some(e.to_string());
                    return;
                }
            },
        };
        match outcome {
            Ok(entry) => self.entry = Some(EntryView::from_entry(&entry)),
            Err(e) => self.entry_error = Some(e.to_string()),
        }
    }
}

/// Whether a dropped file looks like an OpenPGP key by extension.
fn is_keyish(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let e = e.to_ascii_lowercase();
            matches!(e.as_str(), "asc" | "gpg" | "pgp" | "key")
        })
        .unwrap_or(false)
}

impl App {
    /// The navigation rail: the mark, the four destinations, and the theme
    /// toggle at the foot.
    fn rail(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.y = theme::space::TIGHT;
        if self.rail_tex.is_none() {
            self.rail_tex = crate::appicon::rail_texture(ui.ctx());
        }
        theme::rail_head(ui, self.rail_tex.as_ref());

        let s = i18n::s();
        let labels = [s.passwords, s.keys, s.sync, s.settings];
        for (screen, label) in Screen::ALL.into_iter().zip(labels) {
            let selected = self.screen == screen;
            let resp = theme::rail_item(ui, label, selected, |painter, center, color| {
                paint_rail_icon(painter, center, color, screen);
            });
            if resp.clicked() {
                self.screen = screen;
            }
        }

        // Push the theme toggle to the foot.
        let remaining = ui.available_height() - 44.0;
        if remaining > 0.0 {
            ui.add_space(remaining);
        }
        ui.vertical_centered(|ui| {
            let is_dark = ui.visuals().dark_mode;
            let resp = theme::rail_icon_button(ui, |painter, center, color| {
                paint_moon(painter, center, color);
            });
            if resp.clicked() {
                // Toggle explicit light/dark (leaving Auto behind on purpose).
                let next = if is_dark {
                    ThemeChoice::Light
                } else {
                    ThemeChoice::Dark
                };
                self.set_theme(next, ui.ctx());
            }
            resp.on_hover_text(if is_dark { s.theme_light } else { s.theme_dark });
        });
    }

    /// Paint the current screen into the content area.
    fn content(&mut self, ui: &mut egui::Ui) {
        match self.screen {
            Screen::Passwords => panels::passwords::screen(ui, self),
            Screen::Keys => panels::keys::screen(ui, self),
            Screen::Sync => panels::sync::screen(ui, self),
            Screen::Settings => panels::settings::screen(ui, self),
        }
        // The per-file conflict prompt floats over any screen while a sync waits.
        panels::sync::conflict_window(ui.ctx(), self);
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, PREFS_KEY, &self.prefs());
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        i18n::set(self.lang);
        self.poll_jobs();
        self.expire_passphrase_if_stale();
        self.tick_clipboard();
        self.handle_shortcuts(ctx);
        self.handle_drops(ctx);

        // Keep the TOTP ring, clipboard countdown, and any running job live.
        let animating = self.busy
            || self.clip_pending.is_some()
            || self
                .entry
                .as_ref()
                .map(|e| e.otp.is_some())
                .unwrap_or(false);
        if animating {
            ctx.request_repaint_after(Duration::from_millis(300));
        }
    }

    // eframe 0.35: `ui` receives a Ui already filling the whole viewport; we
    // carve the rail and the content out of it with `show_inside`.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // First run takes over the whole window (no rail) until done or skipped.
        if self.onboarding.is_some() {
            egui::CentralPanel::default().show(ui, |ui| {
                panels::onboarding::wizard(ui, self);
            });
            return;
        }

        egui::Panel::left("rail")
            .exact_size(theme::RAIL_WIDTH)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::default().fill(theme::RAIL_BG))
            .show(ui, |ui| self.rail(ui));

        egui::CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(egui::Margin {
                        left: 26,
                        right: 26,
                        top: 22,
                        bottom: 26,
                    }),
            )
            .show(ui, |ui| self.content(ui));

        // A drop hint while files hover over the window.
        let hovering = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
        if hovering {
            paint_drop_overlay(ui.ctx());
        }
    }
}

/// The rail icon for a destination: Lucide glyphs for three, a painted refresh
/// loop for Sync (the subset has no refresh glyph).
fn paint_rail_icon(
    painter: &egui::Painter,
    center: egui::Pos2,
    color: egui::Color32,
    screen: Screen,
) {
    match screen {
        Screen::Passwords => theme::paint_glyph(painter, center, color, theme::icon::LOCK, 22.0),
        Screen::Keys => theme::paint_glyph(painter, center, color, theme::icon::KEY, 22.0),
        Screen::Sync => theme::paint_refresh(painter, center, color, 22.0),
        Screen::Settings => theme::paint_glyph(painter, center, color, theme::icon::GEAR, 22.0),
    }
}

/// A small crescent-moon mark for the theme toggle, painted (no moon glyph in
/// the subset): a filled disc with an offset disc punched out via the rail bg.
fn paint_moon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let r = 8.0;
    painter.circle_filled(center, r, color);
    painter.circle_filled(
        egui::Pos2::new(center.x + 4.0, center.y - 3.0),
        r,
        theme::RAIL_BG,
    );
}

/// A translucent overlay with a message while files are dragged over the window.
fn paint_drop_overlay(ctx: &egui::Context) {
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("drop-overlay"),
    ));
    painter.rect_filled(screen, 0.0, theme::ORANGE_CORE.gamma_multiply(0.14));
    painter.text(
        screen.center(),
        egui::Align2::CENTER_CENTER,
        i18n::s().drop_hint,
        theme::semibold(20.0),
        theme::ORANGE_DEEP,
    );
}

#[cfg(test)]
mod smoke {
    use super::*;

    /// Build the app headlessly and paint the rail plus every screen for one
    /// frame each. This exercises the whole new shell — rail items, cards, the
    /// painted icons and rings, the segmented control, and all seven i18n
    /// tables — so a layout or painter panic (a bad allocation, an out-of-range
    /// index, a missing glyph path) is caught without a display.
    #[test]
    fn every_screen_paints_without_panicking() {
        let ctx = egui::Context::default();
        crate::theme::install_fonts(&ctx);
        crate::theme::install_style(&ctx);
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = App::new(&cc);

        // First-run wizard, across every step and both formats.
        for format in [StoreFormat::Pass, StoreFormat::Passage] {
            for step in [
                OnbStep::Welcome,
                OnbStep::Keys,
                OnbStep::Backup,
                OnbStep::Store,
            ] {
                app.onboarding = Some(Onboarding {
                    step,
                    format,
                    recipient: None,
                    identity_ready: true,
                    backed_up: false,
                    store_path: "/tmp/x".into(),
                });
                let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                    panels::onboarding::wizard(ui, &mut app);
                });
            }
        }

        // The main shell: the rail and each destination, in every language.
        app.onboarding = None;
        for lang in Lang::ALL {
            app.set_language(lang);
            for screen in Screen::ALL {
                app.screen = screen;
                let _ = ctx.run_ui(egui::RawInput::default(), |ui| {
                    app.rail(ui);
                    app.content(ui);
                });
            }
        }
    }
}
