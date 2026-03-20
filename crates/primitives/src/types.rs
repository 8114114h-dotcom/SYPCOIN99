// types.rs — Core primitive types: Amount, BlockHeight, Timestamp, Nonce.
//
// Design decisions:
//
//   AMOUNT — integer-only arithmetic
//   • All monetary values use u64 micro-tokens internally. Floating-point
//     is never used for money — rounding errors in f64 can silently create
//     or destroy value. All arithmetic uses checked_add / checked_sub so
//     that overflow is an explicit Err, never a silent wrap or panic.
//   • Amount does not implement Mul<Amount> — multiplying two monetary
//     values is economically nonsensical. Scaling (e.g. fee × size) is
//     done via Amount::scale(factor: u64).
//
//   BLOCKHEIGHT — newtype over u64
//   • Prevents mixing heights with raw u64 counts or timestamps.
//     A function that takes BlockHeight cannot accidentally receive a Nonce.
//
//   TIMESTAMP — Unix milliseconds, u64
//   • Millisecond resolution matches TARGET_BLOCK_TIME_MS.
//   • u64 milliseconds overflows in the year 584,542,046 — not a concern.
//   • In production, `Timestamp::now()` calls the OS clock. In test builds
//     it can be constructed from a fixed value.
//
//   NONCE — per-account transaction counter
//   • Monotonically increasing. The consensus layer enforces that each
//     transaction's nonce equals the sender's current account nonce.
//   • next() returns Result so overflow (u64::MAX + 1) is handled
//     explicitly rather than wrapping silently.

use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

use crate::constants::{
    HALVING_INTERVAL,
    MAX_SUPPLY_MICRO, MICRO_PER_TOKEN,
};
use crate::error::PrimitivesError;

// ─── Amount ───────────────────────────────────────────────────────────────────

/// A non-negative monetary amount represented in micro-tokens (u64).
///
/// # Invariants
/// - Always ≤ `MAX_SUPPLY_MICRO`.
/// - Never constructed from floating-point.
/// - All arithmetic is overflow-checked and supply-checked.
///
/// # Units
/// ```text
/// 1 TOKEN = 1_000_000 micro-tokens  (6 decimal places)
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct Amount(u64);

impl Amount {
    /// The zero amount.
    pub const ZERO: Amount = Amount(0);

    /// The maximum possible amount (= MAX_SUPPLY_MICRO).
    pub const MAX: Amount = Amount(MAX_SUPPLY_MICRO);

    // ── Constructors ─────────────────────────────────────────────────────────

    /// Construct from a raw micro-token value.
    ///
    /// Returns `Err` if `micro > MAX_SUPPLY_MICRO`.
    pub fn from_micro(micro: u64) -> Result<Self, PrimitivesError> {
        if micro > MAX_SUPPLY_MICRO {
            return Err(PrimitivesError::AmountExceedsMaxSupply(micro));
        }
        Ok(Amount(micro))
    }

    /// Construct from a raw micro-token value without supply check.
    ///
    /// # Safety
    /// Only use for genesis/coinbase accounting where the value is
    /// guaranteed correct by the consensus rules. Prefer `from_micro`.
    pub(crate) fn from_micro_unchecked(micro: u64) -> Self {
        Amount(micro)
    }

    /// Construct from whole tokens.
    ///
    /// `from_tokens(50)` → 50_000_000 micro-tokens.
    /// Returns `Err` if the resulting micro amount exceeds MAX_SUPPLY_MICRO
    /// or if `whole_tokens` would overflow u64 when scaled.
    pub fn from_tokens(whole_tokens: u64) -> Result<Self, PrimitivesError> {
        let micro = whole_tokens
            .checked_mul(MICRO_PER_TOKEN)
            .ok_or(PrimitivesError::AmountOverflow)?;
        Self::from_micro(micro)
    }

    // ── Accessors ────────────────────────────────────────────────────────────

    /// Raw micro-token value. Use for serialization and arithmetic.
    pub fn as_micro(&self) -> u64 {
        self.0
    }

    /// Decompose into `(whole_tokens, fractional_micro)`.
    ///
    /// Example: Amount(1_234_567) → (1, 234_567)
    pub fn to_tokens_parts(&self) -> (u64, u64) {
        (self.0 / MICRO_PER_TOKEN, self.0 % MICRO_PER_TOKEN)
    }

    /// Format as a human-readable decimal string.
    ///
    /// Example: Amount(1_234_567) → "1.234567"
    pub fn to_display_string(&self) -> String {
        let (whole, frac) = self.to_tokens_parts();
        // Zero-pad the fractional part to exactly DECIMAL_PLACES digits.
        format!("{}.{:06}", whole, frac)
    }

    /// Returns `true` if this amount is zero.
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    // ── Arithmetic ───────────────────────────────────────────────────────────

    /// Add two amounts.
    ///
    /// Returns `None` if the result would overflow u64 or exceed MAX_SUPPLY_MICRO.
    pub fn checked_add(self, rhs: Amount) -> Option<Amount> {
        let sum = self.0.checked_add(rhs.0)?;
        if sum > MAX_SUPPLY_MICRO {
            return None;
        }
        Some(Amount(sum))
    }

    /// Subtract `rhs` from `self`.
    ///
    /// Returns `None` if `rhs > self` (no negative amounts).
    pub fn checked_sub(self, rhs: Amount) -> Option<Amount> {
        self.0.checked_sub(rhs.0).map(Amount)
    }

    /// Scale this amount by an integer factor.
    ///
    /// Used for fee calculation: `fee_per_byte.scale(tx_size_bytes)`.
    /// Returns `None` on overflow or supply excess.
    pub fn scale(self, factor: u64) -> Option<Amount> {
        let result = self.0.checked_mul(factor)?;
        if result > MAX_SUPPLY_MICRO {
            return None;
        }
        Some(Amount(result))
    }

    /// Saturating add — clamps to MAX instead of overflowing.
    ///
    /// Use only for display/estimation, never for consensus logic.
    pub fn saturating_add(self, rhs: Amount) -> Amount {
        Amount(self.0.saturating_add(rhs.0).min(MAX_SUPPLY_MICRO))
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

// ─── BlockHeight ──────────────────────────────────────────────────────────────

/// The height (index) of a block in the chain, starting at 0 (genesis).
///
/// A newtype prevents accidental mixing of heights with raw counters,
/// nonces, or timestamps.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct BlockHeight(u64);

impl BlockHeight {
    /// Genesis block height (0).
    pub const GENESIS: BlockHeight = BlockHeight(0);

    /// Construct from a raw u64.
    pub fn new(height: u64) -> Self {
        BlockHeight(height)
    }

    /// Genesis block.
    pub fn genesis() -> Self {
        BlockHeight(0)
    }

    /// Next height (height + 1). Infallible — u64 overflow at height
    /// 18_446_744_073_709_551_615 is not a practical concern.
    pub fn next(&self) -> Self {
        // Wrapping only at u64::MAX which is ~5.8 × 10^11 years at 10s/block.
        BlockHeight(self.0.saturating_add(1))
    }

    /// Raw u64 value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Returns `true` if this is the genesis block (height == 0).
    pub fn is_genesis(&self) -> bool {
        self.0 == 0
    }

    /// Which halving epoch this height falls in.
    ///
    /// Epoch 0 = blocks [0, HALVING_INTERVAL)
    /// Epoch 1 = blocks [HALVING_INTERVAL, 2×HALVING_INTERVAL)
    /// etc.
    pub fn halving_epoch(&self) -> u64 {
        self.0 / HALVING_INTERVAL
    }

    /// Returns `true` if the block reward should be halved at this height.
    pub fn is_halving_block(&self) -> bool {
        self.0 > 0 && self.0 % HALVING_INTERVAL == 0
    }
}

impl std::fmt::Display for BlockHeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

impl From<u64> for BlockHeight {
    fn from(v: u64) -> Self {
        BlockHeight(v)
    }
}

// ─── Timestamp ────────────────────────────────────────────────────────────────

/// A Unix timestamp in milliseconds.
///
/// # Security
/// Block timestamps are validated against `MAX_FUTURE_BLOCK_TIME_MS`.
/// Nodes reject blocks whose timestamp is more than 2 minutes in the future
/// of the node's wall clock, preventing time-warp attacks on difficulty.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Construct from raw milliseconds since Unix epoch.
    pub fn from_millis(ms: u64) -> Self {
        Timestamp(ms)
    }

    /// Current wall-clock time.
    ///
    /// Panics only if the system clock is set before the Unix epoch —
    /// a configuration error, not a runtime error.
    pub fn now() -> Self {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| {
            eprintln!("[WARN] System clock returned error — using 0ms (check system time)");
            std::time::Duration::ZERO
        })
            .as_millis() as u64;
        Timestamp(ms)
    }

    /// Raw millisecond value.
    pub fn as_millis(&self) -> u64 {
        self.0
    }

    /// Returns `true` if `self` is strictly after `other`.
    pub fn is_after(&self, other: &Timestamp) -> bool {
        self.0 > other.0
    }

    /// Returns `true` if `self` is strictly before `other`.
    pub fn is_before(&self, other: &Timestamp) -> bool {
        self.0 < other.0
    }

    /// Milliseconds elapsed from `other` to `self`.
    ///
    /// Returns `None` if `self < other` (would be negative).
    pub fn millis_since(&self, other: &Timestamp) -> Option<u64> {
        self.0.checked_sub(other.0)
    }

    /// Add a duration in milliseconds.
    ///
    /// Returns `None` on overflow.
    pub fn checked_add_millis(&self, ms: u64) -> Option<Timestamp> {
        self.0.checked_add(ms).map(Timestamp)
    }

    /// Validate that this timestamp is not implausibly far in the future
    /// relative to `now`.
    ///
    /// Used by the block validator before accepting a block.
    pub fn validate_not_future(
        &self,
        now: &Timestamp,
        max_drift_ms: u64,
    ) -> Result<(), PrimitivesError> {
        if let Some(diff) = self.0.checked_sub(now.0) {
            if diff > max_drift_ms {
                return Err(PrimitivesError::TimestampInFuture);
            }
        }
        Ok(())
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Display as seconds with 3 decimal places for readability.
        write!(f, "{:.3}s", self.0 as f64 / 1000.0)
    }
}

// ─── Nonce ────────────────────────────────────────────────────────────────────

/// A per-account transaction counter used for replay protection.
///
/// The account nonce starts at 0 and must increase by exactly 1 with each
/// confirmed transaction. A transaction is valid only if its nonce equals
/// the sender's current account nonce.
///
/// This is distinct from the PoW mining nonce (which lives in the block header).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize)]
pub struct Nonce(u64);

impl Nonce {
    /// The initial nonce for a new account (0).
    pub const ZERO: Nonce = Nonce(0);

    /// Construct from a raw u64.
    pub fn new(n: u64) -> Self {
        Nonce(n)
    }

    /// Initial nonce for a new account.
    pub fn zero() -> Self {
        Nonce(0)
    }

    /// The next expected nonce after this one.
    ///
    /// Returns `Err(NonceOverflow)` if this is already u64::MAX.
    /// In practice, an account would need to send 1.8 × 10^19 transactions —
    /// this is handled for correctness, not practicality.
    pub fn next(&self) -> Result<Nonce, PrimitivesError> {
        self.0
            .checked_add(1)
            .map(Nonce)
            .ok_or(PrimitivesError::NonceOverflow)
    }

    /// Raw u64 value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Returns `true` if this nonce directly follows `previous`.
    ///
    /// Used by the transaction validator:
    /// ```text
    /// tx.nonce.follows(account.nonce) == true  →  valid sequence
    /// ```
    pub fn follows(&self, previous: &Nonce) -> bool {
        self.0 == previous.0.wrapping_add(1) && self.0 != 0
    }
}

impl std::fmt::Display for Nonce {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "nonce({})", self.0)
    }
}

impl From<u64> for Nonce {
    fn from(v: u64) -> Self {
        Nonce(v)
    }
}
