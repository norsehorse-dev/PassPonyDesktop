//! A password generator. Non-GUI and pure so it is unit-testable; the editor
//! panel and the `generate` verb both call it.

use rand::Rng;

const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
// A conservative symbol set: no quotes/backslashes/spaces that tend to trip up
// shells, CSV, or careless copy-paste.
const SYMBOLS: &str = "!@#$%^&*()-_=+[]{};:,.?";

#[derive(Debug, Clone, Copy)]
pub struct PwOptions {
    pub len: usize,
    pub letters: bool,
    pub digits: bool,
    pub symbols: bool,
}

impl Default for PwOptions {
    fn default() -> Self {
        Self {
            len: 20,
            letters: true,
            digits: true,
            symbols: true,
        }
    }
}

/// Generate a password from the selected character classes. If every class is
/// off, falls back to letters so it never returns an empty or fixed string.
#[must_use]
pub fn generate(opts: &PwOptions) -> String {
    let mut charset = String::new();
    if opts.letters {
        charset.push_str(LOWER);
        charset.push_str(UPPER);
    }
    if opts.digits {
        charset.push_str(DIGITS);
    }
    if opts.symbols {
        charset.push_str(SYMBOLS);
    }
    if charset.is_empty() {
        charset.push_str(LOWER);
        charset.push_str(UPPER);
    }
    let chars: Vec<char> = charset.chars().collect();
    let len = opts.len.max(1);
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| chars[rng.gen_range(0..chars.len())])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_length() {
        let pw = generate(&PwOptions {
            len: 32,
            ..Default::default()
        });
        assert_eq!(pw.chars().count(), 32);
    }

    #[test]
    fn digits_only_yields_digits() {
        let pw = generate(&PwOptions {
            len: 40,
            letters: false,
            digits: true,
            symbols: false,
        });
        assert!(pw.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn all_classes_off_falls_back_to_letters() {
        let pw = generate(&PwOptions {
            len: 16,
            letters: false,
            digits: false,
            symbols: false,
        });
        assert_eq!(pw.chars().count(), 16);
        assert!(pw.chars().all(|c| c.is_ascii_alphabetic()));
    }
}
