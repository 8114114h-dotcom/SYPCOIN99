// keys/private_key.rs — PrivateKey newtype.
//
// Security decisions:
//
//   ZEROIZE ON DROP
//   • `#[derive(ZeroizeOnDrop)]` injects a `Drop` impl that calls
//     `self.0.zeroize()` before the allocator reclaims the memory.
//     This overwrites the 32-byte scalar with zeros even during panic
//     unwinds, preventing key material from appearing in crash dumps.
//
//   • `#[derive(Zeroize)]` additionally allows other code in this crate
//     (e.g., keypair.rs) to call `.zeroize()` on a PrivateKey explicitly
//     before drop if an early wipe is needed.
//
//   NO CLONE / COPY / DEBUG / DISPLAY / SERIALIZE
//   • These traits are deliberately absent. Each would create an
//     additional copy of the scalar or expose it to output streams:
//       - Clone/Copy → second live copy, only one is zeroized
//       - Debug/Display → key bytes appear in logs / panic messages
//       - Serialize → key bytes written to disk / network
//   • The absence is enforced at compile time. Any attempt to add these
//     traits must be rejected in code review.
//
//   PUB(CRATE) INNER FIELD
//   • The inner [u8; 32] is `pub(crate)` so that only `signing/signer.rs`
//     can read the raw bytes (to reconstruct a dalek SigningKey transiently
//     during signing). Downstream crates never see the scalar.

use zeroize::{Zeroize, ZeroizeOnDrop};

/// A 32-byte Ed25519 private key scalar.
///
/// # Guarantees
/// - Memory is zeroed on drop (even during panic unwinds).
/// - At most one live copy exists at any time (no `Clone`).
/// - Never appears in logs or on the wire (no `Debug`, no `Serialize`).
/// - Never accessible outside this crate (field is `pub(crate)`).
#[derive(Zeroize, ZeroizeOnDrop)]
pub(crate) struct PrivateKey(pub(crate) [u8; 32]);
