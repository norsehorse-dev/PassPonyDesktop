//! The desktop's native crypto backends for pass-core's `CryptoBackend` seam.
//!
//! pass-core is crypto-free by design: it hands ciphertext and a resolved
//! recipient set to a `CryptoBackend`. On the phones that boundary is crossed
//! by a UniFFI callback into Kotlin/Swift; here it is an ordinary Rust trait
//! impl, because the crypto is in the same language as the core.
//!
//! - [`pass`] backs the pass (OpenPGP) format over rpgp.
//! - [`age`](age_backend) backs the passage (age) format over the `age` crate.
//!
//! Both were validated against real gpg-produced stores and cross-tool round
//! trips in the Phase 0 spike; see PHASE0_SPIKE_RESULTS.md.

pub mod age_backend;
pub mod keystore;
pub mod pgp_backend;

use pass_core::crypto::CryptoBackend;
use pass_core::store::StoreFormat;

pub use age_backend::AgeBackend;
pub use keystore::PgpKeyStore;
pub use pgp_backend::PgpBackend;

/// A backend loaded for a given store format, ready to hand to pass-core.
pub enum Backend {
    Pass(PgpBackend),
    Passage(AgeBackend),
}

impl Backend {
    /// Borrow the inner backend as the trait object pass-core takes.
    #[must_use]
    pub fn as_crypto(&self) -> &dyn CryptoBackend {
        match self {
            Backend::Pass(b) => b,
            Backend::Passage(b) => b,
        }
    }

    /// Which format this backend serves.
    #[must_use]
    pub fn format(&self) -> StoreFormat {
        match self {
            Backend::Pass(_) => StoreFormat::Pass,
            Backend::Passage(_) => StoreFormat::Passage,
        }
    }
}
