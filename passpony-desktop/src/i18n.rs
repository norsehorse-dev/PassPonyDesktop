//! Localization. The seven PassPony languages, one struct of UI strings per
//! language, and a thread-local "current" pointer so any panel can read strings
//! with `i18n::s()` without threading state through every call. The UI runs on
//! one thread, so the thread-local is set from `app.lang` at the top of each
//! frame.
//!
//! Short UI strings are translated here. The long-form Help FAQ and Security
//! prose (in `panels::settings`) stay English for now, sourced later from the
//! phones' professional catalogs rather than machine-translated in a security
//! app; `Strings::help_english_note` labels that.

use std::cell::Cell;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Es,
    Fr,
    De,
    Zh,
    Pt,
    Ru,
}

impl Lang {
    pub const ALL: [Lang; 7] = [
        Lang::En,
        Lang::Es,
        Lang::Fr,
        Lang::De,
        Lang::Zh,
        Lang::Pt,
        Lang::Ru,
    ];

    /// Stable code stored on disk and shared with the phone catalogs.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Es => "es",
            Lang::Fr => "fr",
            Lang::De => "de",
            Lang::Zh => "zh-Hans",
            Lang::Pt => "pt-BR",
            Lang::Ru => "ru",
        }
    }

    /// The language's own name, for the picker.
    #[must_use]
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::Fr => "Français",
            Lang::De => "Deutsch",
            Lang::Zh => "简体中文",
            Lang::Pt => "Português (BR)",
            Lang::Ru => "Русский",
        }
    }

    #[must_use]
    pub fn from_code(code: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.code() == code)
    }

    #[must_use]
    pub fn strings(self) -> &'static Strings {
        match self {
            Lang::En => &EN,
            Lang::Es => &ES,
            Lang::Fr => &FR,
            Lang::De => &DE,
            Lang::Zh => &ZH,
            Lang::Pt => &PT,
            Lang::Ru => &RU,
        }
    }

    /// The long-form Settings prose (Help FAQ, Security, licenses) for this
    /// language. Kept apart from `Strings` because it is arrays of paragraphs,
    /// not scalar labels; the tables live in `i18n_help.rs`.
    #[must_use]
    pub fn long(self) -> &'static LongForm {
        match self {
            Lang::En => &EN_LONG,
            Lang::Es => &ES_LONG,
            Lang::Fr => &FR_LONG,
            Lang::De => &DE_LONG,
            Lang::Zh => &ZH_LONG,
            Lang::Pt => &PT_LONG,
            Lang::Ru => &RU_LONG,
        }
    }
}

thread_local! {
    static CURRENT: Cell<&'static Strings> = const { Cell::new(&EN) };
    static CURRENT_LONG: Cell<&'static LongForm> = const { Cell::new(&EN_LONG) };
}

/// Set the active language for this thread (call once per frame from `app`).
pub fn set(lang: Lang) {
    CURRENT.with(|c| c.set(lang.strings()));
    CURRENT_LONG.with(|c| c.set(lang.long()));
}

/// The active language's short strings.
#[must_use]
pub fn s() -> &'static Strings {
    CURRENT.with(Cell::get)
}

/// The active language's long-form Settings prose.
#[must_use]
pub fn long() -> &'static LongForm {
    CURRENT_LONG.with(Cell::get)
}

/// The long-form Settings content, localized per language. Each `(&str, &str)`
/// in `faq`/`security` is a (heading, body) pair; `crates` is (what it is,
/// its license). Product names, file extensions, SPDX license ids and paths
/// (`pass`, `passage`, `.gpg-id`, `otpauth://`, `age`, `rpgp`) stay verbatim.
pub struct LongForm {
    pub faq: &'static [(&'static str, &'static str)],
    pub security: &'static [(&'static str, &'static str)],
    pub security_not_title: &'static str,
    pub security_not: &'static [&'static str],
    pub crates: &'static [(&'static str, &'static str)],
}

/// Every short UI string, one field per message. Placeholders use `{}` and are
/// filled by the caller with `format!`/`str::replace`.
pub struct Strings {
    // toolbar / nav
    pub fmt_pass: &'static str,
    pub fmt_passage: &'static str,
    pub open_store: &'static str,
    pub default_store: &'static str,
    pub new_entry: &'static str,
    pub keys: &'static str,
    pub sync: &'static str,
    pub settings: &'static str,
    pub search: &'static str,
    // list / detail
    pub open_a_store: &'static str,
    pub entries_count: &'static str, // "{} entries"
    pub select_entry: &'static str,
    pub opening: &'static str,
    // keys section
    pub keys_identities: &'static str,
    pub import_pgp: &'static str,
    pub generate_age: &'static str,
    pub passphrase: &'static str,
    pub passphrase_hint: &'static str,
    pub no_pass_keys: &'static str,
    pub pass_keys: &'static str, // "pass keys: {}"
    // entry detail
    pub edit: &'static str,
    pub delete: &'static str,
    pub confirm_delete: &'static str,
    pub cancel: &'static str,
    pub password: &'static str,
    pub reveal: &'static str,
    pub copy: &'static str,
    pub copy_code: &'static str,
    pub clipboard_clears: &'static str, // "Clipboard clears in {}s."
    pub decrypt_hint: &'static str,
    // editor
    pub new_entry_title: &'static str,
    pub edit_entry_title: &'static str,
    pub name: &'static str,
    pub generate: &'static str,
    pub new_password: &'static str,
    pub content_label: &'static str,
    pub save: &'static str,
    // sync
    pub sync_title: &'static str,
    pub remote: &'static str,       // "remote: {}"
    pub ahead_behind: &'static str, // "{} ahead · {} behind"
    pub uncommitted: &'static str,  // " · uncommitted changes"
    pub not_a_repo: &'static str,
    pub open_or_clone: &'static str,
    pub remote_url: &'static str,
    pub user: &'static str,
    pub token: &'static str,
    pub https_note: &'static str,
    pub clone: &'static str,
    pub publish: &'static str,
    pub fetch: &'static str,
    pub push: &'static str,
    pub conflict_title: &'static str,
    pub conflict_both: &'static str, // "Both sides changed: {}"
    pub conflict_which: &'static str,
    pub keep_mine: &'static str,
    pub keep_remote: &'static str,
    pub keep_both: &'static str,
    pub keep_both_note: &'static str,
    // onboarding
    pub welcome: &'static str,
    pub pick_format: &'static str,
    pub next: &'static str,
    pub skip_setup: &'static str,
    pub back: &'static str,
    pub gen_identity_desc: &'static str,
    pub generate_my_identity: &'static str,
    pub your_recipient: &'static str,
    pub already_have_identity: &'static str,
    pub import_key_desc: &'static str,
    pub imported_key: &'static str,
    pub backup_warn: &'static str,
    pub backup_desc: &'static str,
    pub save_a_copy: &'static str,
    pub backed_up: &'static str,
    pub skip_for_now: &'static str,
    pub where_store: &'static str,
    pub create_store: &'static str,
    pub clone_instead: &'static str,
    // settings
    pub appearance: &'static str,
    pub theme_auto: &'static str,
    pub theme_light: &'static str,
    pub theme_dark: &'static str,
    pub theme_note: &'static str,
    pub language: &'static str,
    pub language_note: &'static str,
    pub storage: &'static str,
    pub show_in_folder: &'static str,
    pub storage_note: &'static str,
    pub reencrypt_title: &'static str,
    pub reencrypt_note: &'static str,
    pub reencrypt_run: &'static str,
    pub backup_title: &'static str,
    pub backup_settings_note: &'static str,
    pub export_keys: &'static str,
    pub about: &'static str,
    pub about_desc: &'static str,
    pub source_github: &'static str,
    pub license_apache: &'static str,
    pub help: &'static str,
    pub help_english_note: &'static str,
    pub security_title: &'static str,
    pub licenses_title: &'static str,
    pub licenses_note: &'static str,
    pub panic_title: &'static str,
    pub panic_desc: &'static str,
    pub panic_button: &'static str,
    pub panic_confirm: &'static str,
    pub family_title: &'static str,
    pub family_hub_label: &'static str,
    pub family_note: &'static str,
    // rail shell / passwords screen
    pub passwords: &'static str,
    pub drop_hint: &'static str,
    pub seg_pass: &'static str,
    pub seg_passage: &'static str,
    pub store_label: &'static str,
    pub search_placeholder: &'static str,
    pub one_time_code: &'static str,
    pub hide: &'static str,
    // keys screen
    pub keys_title: &'static str,
    pub keys_sub: &'static str,
    pub pass_keys_section: &'static str,
    pub imported: &'static str,
    pub passphrase_placeholder: &'static str,
    pub passphrase_note: &'static str,
    pub passage_section: &'static str,
    pub age_identity_present: &'static str,
    pub this_machine: &'static str,
    pub no_age_identity: &'static str,
    pub back_up_export: &'static str,
    pub sync_not_keys: &'static str,
    pub keep_key_safe: &'static str,
    // sync screen
    pub sync_sub: &'static str,
    pub ahead: &'static str,
    pub behind: &'static str,
    pub working_tree: &'static str,
    pub dirty_tree: &'static str,
    pub clean_tree: &'static str,
    pub remote_label: &'static str,
    pub remote_section: &'static str,
}

include!("i18n_data.rs");
include!("i18n_help.rs");
