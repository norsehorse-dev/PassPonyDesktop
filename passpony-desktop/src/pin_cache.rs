//! The YubiKey PIN, remembered for the same five minutes as a pass key's
//! passphrase, so opening several entries in a row asks for it once (the
//! touch is still needed every time).
//!
//! A PIN is kept only after an entry actually opened with it, and only for
//! the prompt it answered: age-plugin-yubikey names the YubiKey's serial in
//! it, so one key's PIN never answers another's. A remembered PIN is sent at
//! most once per open, and an open that fails with it forgets it, so a
//! changed PIN costs one wrong try rather than burning the PIV retries.

use std::time::{Duration, Instant};

use zeroize::Zeroizing;

/// One remembered PIN.
struct Remembered {
    prompt: String,
    pin: Zeroizing<String>,
    at: Instant,
}

/// The PIN cache, and what the open in progress did with it.
pub struct PinCache {
    ttl: Duration,
    remembered: Option<Remembered>,
    /// The PIN typed during the open in progress, kept if it succeeds.
    typed: Option<(String, Zeroizing<String>)>,
    /// Whether the open in progress was answered from the cache.
    used: bool,
}

impl PinCache {
    #[must_use]
    pub fn new(ttl: Duration) -> Self {
        PinCache {
            ttl,
            remembered: None,
            typed: None,
            used: false,
        }
    }

    /// An open is starting: forget what the last one did.
    pub fn begin(&mut self) {
        self.typed = None;
        self.used = false;
    }

    /// The plugin asks `prompt`: the PIN to send without asking the user, if
    /// one is remembered for it and this open hasn't used it yet. Asked again
    /// after using it means it was refused, so it is forgotten.
    pub fn answer(&mut self, prompt: &str, now: Instant) -> Option<Zeroizing<String>> {
        self.expire(now);
        if self.used {
            self.remembered = None;
            return None;
        }
        let r = self.remembered.as_ref().filter(|r| r.prompt == prompt)?;
        self.used = true;
        Some(r.pin.clone())
    }

    /// The user typed `pin` for `prompt`.
    pub fn typed(&mut self, prompt: &str, pin: Zeroizing<String>) {
        self.typed = Some((prompt.to_owned(), pin));
    }

    /// The open finished: keep the typed PIN if it worked, forget a
    /// remembered one that didn't.
    pub fn finish(&mut self, opened: bool, now: Instant) {
        let typed = self.typed.take();
        if opened {
            if let Some((prompt, pin)) = typed {
                self.remembered = Some(Remembered {
                    prompt,
                    pin,
                    at: now,
                });
            }
        } else if self.used {
            self.remembered = None;
        }
        self.used = false;
    }

    /// Forget the PIN once its five minutes are up.
    pub fn expire(&mut self, now: Instant) {
        if self
            .remembered
            .as_ref()
            .is_some_and(|r| now.duration_since(r.at) >= self.ttl)
        {
            self.remembered = None;
        }
    }

    /// Forget everything (store closed, panic wipe).
    pub fn clear(&mut self) {
        self.remembered = None;
        self.typed = None;
        self.used = false;
    }

    /// Whether a PIN is remembered right now.
    #[must_use]
    pub fn holds_pin(&self) -> bool {
        self.remembered.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TTL: Duration = Duration::from_secs(300);
    const P1: &str = "Enter PIN for YubiKey with serial 1";
    const P2: &str = "Enter PIN for YubiKey with serial 2";

    fn pin(s: &str) -> Zeroizing<String> {
        Zeroizing::new(s.to_owned())
    }

    /// One open where the user types `typed` and it `opened` or not.
    fn open_typing(c: &mut PinCache, prompt: &str, typed: &str, opened: bool, now: Instant) {
        c.begin();
        assert!(c.answer(prompt, now).is_none());
        c.typed(prompt, pin(typed));
        c.finish(opened, now);
    }

    #[test]
    fn remembers_a_pin_that_opened_an_entry() {
        let t0 = Instant::now();
        let mut c = PinCache::new(TTL);
        open_typing(&mut c, P1, "123456", true, t0);
        assert!(c.holds_pin());

        c.begin();
        assert_eq!(
            c.answer(P1, t0 + Duration::from_secs(60))
                .as_deref()
                .map(String::as_str),
            Some("123456")
        );
        c.finish(true, t0 + Duration::from_secs(60));
        assert!(c.holds_pin());
    }

    #[test]
    fn never_remembers_a_pin_that_failed_or_was_not_sent() {
        let t0 = Instant::now();
        let mut c = PinCache::new(TTL);
        open_typing(&mut c, P1, "000000", false, t0);
        assert!(!c.holds_pin());

        c.begin();
        assert!(c.answer(P1, t0).is_none());
        c.finish(false, t0); // cancelled: nothing typed
        assert!(!c.holds_pin());
    }

    #[test]
    fn only_answers_its_own_yubikey() {
        let t0 = Instant::now();
        let mut c = PinCache::new(TTL);
        open_typing(&mut c, P1, "111111", true, t0);
        c.begin();
        assert!(c.answer(P2, t0).is_none());
    }

    #[test]
    fn expires_five_minutes_after_it_was_typed() {
        let t0 = Instant::now();
        let mut c = PinCache::new(TTL);
        open_typing(&mut c, P1, "123456", true, t0);
        // Using it doesn't extend it, like the passphrase.
        c.begin();
        assert!(c.answer(P1, t0 + Duration::from_secs(200)).is_some());
        c.finish(true, t0 + Duration::from_secs(200));
        c.begin();
        assert!(c.answer(P1, t0 + TTL).is_none());
        assert!(!c.holds_pin());
    }

    #[test]
    fn a_refused_pin_is_sent_once_then_forgotten() {
        let t0 = Instant::now();
        let mut c = PinCache::new(TTL);
        open_typing(&mut c, P1, "123456", true, t0);

        // The PIN was changed elsewhere: the plugin asks again after the
        // cached answer, and the user gets the prompt.
        c.begin();
        assert!(c.answer(P1, t0).is_some());
        assert!(c.answer(P1, t0).is_none());
        assert!(!c.holds_pin());
        c.typed(P1, pin("654321"));
        c.finish(true, t0);
        c.begin();
        assert_eq!(
            c.answer(P1, t0).as_deref().map(String::as_str),
            Some("654321")
        );
    }

    #[test]
    fn an_open_that_fails_with_the_cached_pin_forgets_it() {
        let t0 = Instant::now();
        let mut c = PinCache::new(TTL);
        open_typing(&mut c, P1, "123456", true, t0);
        c.begin();
        assert!(c.answer(P1, t0).is_some());
        c.finish(false, t0);
        assert!(!c.holds_pin());
    }

    #[test]
    fn clear_forgets_everything() {
        let t0 = Instant::now();
        let mut c = PinCache::new(TTL);
        open_typing(&mut c, P1, "123456", true, t0);
        c.clear();
        assert!(!c.holds_pin());
    }
}
