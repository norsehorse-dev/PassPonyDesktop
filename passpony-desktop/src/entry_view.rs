//! A parsed, display-ready view of a decrypted entry.
//!
//! Kept out of the GUI modules on purpose: it depends only on pass-core and
//! zeroize, so the parsing and formatting are unit-testable without a display
//! stack. The password is held in a zeroizing buffer and wiped on drop.

use pass_core::entry::Entry;
use pass_core::totp::Totp;
use zeroize::Zeroizing;

/// The pieces the detail panel shows: the password, the `key: value` fields,
/// and a parsed TOTP if the entry carries an `otpauth://` line.
pub struct EntryView {
    pub password: Zeroizing<String>,
    pub fields: Vec<(String, String)>,
    pub otp: Option<Totp>,
    /// The full decrypted entry text, for the raw editor. Zeroized on drop.
    pub raw: Zeroizing<String>,
}

impl EntryView {
    #[must_use]
    pub fn from_entry(entry: &Entry) -> Self {
        let password = Zeroizing::new(String::from_utf8_lossy(entry.password()).into_owned());
        let fields = entry
            .fields()
            .into_iter()
            .map(|f| (f.key.to_string(), f.value.to_string()))
            .collect();
        let otp = entry.otpauth().and_then(|uri| Totp::from_uri(uri).ok());
        let raw = Zeroizing::new(String::from_utf8_lossy(entry.to_bytes()).into_owned());
        Self {
            password,
            fields,
            otp,
            raw,
        }
    }
}

/// Group a TOTP code into halves for readability: `123456` -> `123 456`,
/// `12345678` -> `1234 5678`. Odd lengths split after the midpoint.
#[must_use]
pub fn group_code(code: &str) -> String {
    let n = code.len();
    if n < 6 {
        return code.to_string();
    }
    let mid = n / 2;
    format!("{} {}", &code[..mid], &code[mid..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_password_fields_and_otp() {
        let raw = b"s3cret\nlogin: kevin\nurl: example.com\notpauth://totp/x?secret=JBSWY3DPEHPK3PXP&period=30".to_vec();
        let view = EntryView::from_entry(&Entry::from_bytes(raw));
        assert_eq!(&*view.password, "s3cret");
        assert!(view
            .fields
            .iter()
            .any(|(k, v)| k == "login" && v == "kevin"));
        assert!(view
            .fields
            .iter()
            .any(|(k, v)| k == "url" && v == "example.com"));
        // the otpauth line is not a field
        assert!(!view.fields.iter().any(|(k, _)| k.starts_with("otpauth")));
        let otp = view.otp.expect("otp parsed");
        // deterministic code at a fixed time
        assert_eq!(otp.code_at(59).len(), 6);
        assert_eq!(otp.period, 30);
    }

    #[test]
    fn entry_without_otp_has_none() {
        let view = EntryView::from_entry(&Entry::from_bytes(b"pw\nlogin: a".to_vec()));
        assert!(view.otp.is_none());
    }

    #[test]
    fn grouping() {
        assert_eq!(group_code("123456"), "123 456");
        assert_eq!(group_code("12345678"), "1234 5678");
        assert_eq!(group_code("1234"), "1234");
    }
}
