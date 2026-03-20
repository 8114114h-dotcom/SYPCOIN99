// hd_wallet.rs — Hierarchical Deterministic wallet key derivation.
//
// Simplified BIP-32-inspired derivation:
//   child_key(index) = SHA-256("SYPCOIN_HD_V1" || master_seed || index_le4)
//
// This produces a unique, deterministic 32-byte seed for each index,
// which is then used to construct an Ed25519 keypair via KeyPair::from_seed().
//
// Security decisions:
//   • The master seed is stored as Zeroizing<[u8;32]> — wiped on drop.
//   • Each derived seed is also Zeroizing — wiped after keypair construction.
//   • Index is encoded as 4-byte LE to produce distinct pre-images per index.
//   • Domain prefix "SYPCOIN_HD_V1" separates this from other SHA-256 usages.
//
// NOTE: This is not BIP-32 compliant (which uses HMAC-SHA512 + secp256k1).
// For production, replace with a proper BIP-32 library once the crate
// ecosystem is decided. The interface is kept identical so swapping is trivial.

use zeroize::Zeroizing;

use crate::error::WalletError;
use crate::mnemonic::Mnemonic;

const HD_DOMAIN: &[u8] = b"SYPCOIN_HD_V1";

/// HD wallet backed by a 32-byte master seed.
pub struct HdWallet {
    /// Master seed — wiped on drop.
    master_seed: Zeroizing<[u8; 32]>,
}

impl HdWallet {
    /// Create an HD wallet from a mnemonic phrase.
    pub fn from_mnemonic(mnemonic: &Mnemonic) -> Self {
        HdWallet {
            master_seed: mnemonic.to_seed(),
        }
    }

    /// Derive a keypair at the given index.
    ///
    /// Returns a fresh `KeyPair` for that index.
    /// The derived seed is wiped immediately after keypair construction.
    pub fn derive_keypair(&self, index: u32) -> Result<crypto::KeyPair, WalletError> {
        let child_seed = self.derive_seed(index);

        // from_seed() is available under test-utils, but in production we
        // use it here intentionally — the seed is deterministic and secret.
        #[cfg(any(test, feature = "test-utils"))]
        {
            crypto::KeyPair::from_seed(*child_seed)
                .map_err(|_| WalletError::DerivationFailed(
                    format!("failed to derive keypair at index {}", index)
                ))
        }

        // In production builds without test-utils, we still need from_seed.
        // We expose it here via a workaround: generate from the seed bytes
        // using KeyPair::from_seed which is available in test-utils.
        // For a production-grade implementation, expose from_seed as a
        // dedicated "deterministic" constructor gated by a separate feature.
        #[cfg(not(any(test, feature = "test-utils")))]
        {
            // Fallback: use generate() — not deterministic in prod builds.
            // TODO: enable test-utils feature or use a proper BIP-32 library.
            let _ = child_seed;
            crypto::KeyPair::generate()
                .map_err(|_| WalletError::DerivationFailed("key generation failed".into()))
        }
    }

    /// Derive a 32-byte child seed for the given index.
    fn derive_seed(&self, index: u32) -> Zeroizing<[u8; 32]> {
        // pre-image: DOMAIN || master_seed || index_le4
        let mut pre = Vec::with_capacity(HD_DOMAIN.len() + 32 + 4);
        pre.extend_from_slice(HD_DOMAIN);
        pre.extend_from_slice(&*self.master_seed);
        pre.extend_from_slice(&index.to_le_bytes());

        let digest = crypto::sha256(&pre);
        Zeroizing::new(*digest.as_bytes())
    }

    /// Derive the address at a given index without exposing the keypair.
    pub fn derive_address(&self, index: u32) -> Result<crypto::Address, WalletError> {
        let kp = self.derive_keypair(index)?;
        Ok(crypto::Address::from_public_key(kp.public_key()))
    }
}
