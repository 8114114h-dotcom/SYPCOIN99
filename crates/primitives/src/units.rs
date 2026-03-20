// units.rs — Unit conversion helpers and monetary policy functions.
//
// Design decisions:
//   • block_reward_at() is pure and deterministic — given a height it always
//     returns the same reward. This is essential: every node must independently
//     compute the same coinbase amount or reject the block.
//
//   • After MAX_HALVINGS halvings the reward is 0 (integer right-shift to zero).
//     This is intentional — miners then earn only transaction fees.
//
//   • is_supply_valid() does a cumulative supply check. It is used by the
//     genesis builder and by tests. The consensus layer enforces supply limits
//     per-block using checked_add on the state's total_supply field.

use crate::constants::{
    HALVING_INTERVAL, INITIAL_BLOCK_REWARD, MAX_HALVINGS, MAX_SUPPLY_MICRO, MICRO_PER_TOKEN,
};
use crate::error::PrimitivesError;
use crate::types::{Amount, BlockHeight};

// ─── Display / parsing helpers ────────────────────────────────────────────────

/// Format a micro-token value as a human-readable decimal string.
///
/// ```text
/// micro_to_display(1_234_567) == "1.234567"
/// micro_to_display(0)         == "0.000000"
/// micro_to_display(500_000)   == "0.500000"
/// ```
pub fn micro_to_display(micro: u64) -> String {
    let whole = micro / MICRO_PER_TOKEN;
    let frac  = micro % MICRO_PER_TOKEN;
    format!("{}.{:06}", whole, frac)
}

/// Parse a decimal string into micro-tokens.
///
/// Accepts:
///   - `"1"`          → 1_000_000
///   - `"1.5"`        → 1_500_000
///   - `"0.000001"`   → 1
///   - `"21000000"`   → 21_000_000_000_000
///
/// Rejects:
///   - More than 6 decimal places
///   - Non-numeric characters
///   - Values exceeding MAX_SUPPLY_MICRO
pub fn display_to_micro(s: &str) -> Result<u64, PrimitivesError> {
    let invalid = || PrimitivesError::InvalidTokenAmount(s.to_owned());

    let (whole_str, frac_str) = match s.split_once('.') {
        Some((w, f)) => (w, f),
        None         => (s, ""),
    };

    // Validate and parse whole part.
    let whole: u64 = if whole_str.is_empty() {
        0
    } else {
        whole_str.parse::<u64>().map_err(|_| invalid())?
    };

    // Validate fractional part: max 6 digits.
    if frac_str.len() > 6 {
        return Err(invalid());
    }
    // Right-pad to exactly 6 digits then parse.
    let frac_padded = format!("{:0<6}", frac_str); // e.g. "5" → "500000"
    let frac: u64 = if frac_padded.is_empty() {
        0
    } else {
        frac_padded.parse::<u64>().map_err(|_| invalid())?
    };

    let micro = whole
        .checked_mul(MICRO_PER_TOKEN)
        .and_then(|w| w.checked_add(frac))
        .ok_or(PrimitivesError::AmountOverflow)?;

    if micro > MAX_SUPPLY_MICRO {
        return Err(PrimitivesError::AmountExceedsMaxSupply(micro));
    }

    Ok(micro)
}

// ─── Monetary policy ──────────────────────────────────────────────────────────

/// Calculate the block reward at a given height.
///
/// The reward halves every `HALVING_INTERVAL` blocks:
/// ```text
/// Height 0–209_999      → 50.000000 tokens
/// Height 210_000–419_999 → 25.000000 tokens
/// Height 420_000–629_999 → 12.500000 tokens
/// ...
/// After MAX_HALVINGS    → 0.000000  tokens  (fees only)
/// ```
///
/// This function is **consensus-critical** and must be deterministic.
/// It is pure — no state, no I/O.
pub fn block_reward_at(height: &BlockHeight) -> Amount {
    let epoch = height.halving_epoch();

    // Once we've exceeded MAX_HALVINGS halvings, the integer shift produces 0.
    if epoch >= MAX_HALVINGS as u64 {
        return Amount::ZERO;
    }

    // INITIAL_BLOCK_REWARD >> epoch performs exact integer halving.
    // This matches Bitcoin's reward schedule precisely.
    let reward = INITIAL_BLOCK_REWARD >> epoch;

    // Safety: reward is always ≤ INITIAL_BLOCK_REWARD ≤ MAX_SUPPLY_MICRO.
    Amount::from_micro_unchecked(reward)
}

/// Returns `true` if `amount` does not exceed MAX_SUPPLY_MICRO.
pub fn is_supply_valid(amount: &Amount) -> bool {
    amount.as_micro() <= MAX_SUPPLY_MICRO
}

/// Calculate the total theoretical supply that could have been minted
/// up to (but not including) `height`.
///
/// Used for genesis validation and supply auditing.
/// Returns `None` on arithmetic overflow (should not happen in practice).
pub fn theoretical_supply_at(height: &BlockHeight) -> Option<Amount> {
    let mut total: u64 = 0;
    let h = height.as_u64();

    // Sum rewards epoch by epoch for efficiency.
    let mut remaining = h;
    let mut epoch: u64 = 0;

    loop {
        if epoch >= MAX_HALVINGS as u64 {
            break;
        }
        let reward = INITIAL_BLOCK_REWARD >> epoch;
        if reward == 0 {
            break;
        }
        let blocks_in_epoch = remaining.min(HALVING_INTERVAL);
        let epoch_supply = reward.checked_mul(blocks_in_epoch)?;
        total = total.checked_add(epoch_supply)?;
        remaining = remaining.saturating_sub(blocks_in_epoch);
        if remaining == 0 {
            break;
        }
        epoch += 1;
    }

    if total > MAX_SUPPLY_MICRO {
        return None;
    }

    Some(Amount::from_micro_unchecked(total))
}
