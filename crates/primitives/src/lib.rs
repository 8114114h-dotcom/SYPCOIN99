// lib.rs — Public API surface for the `primitives` crate.
//
// RULE: Nothing is public unless explicitly re-exported here.
//
// Every other crate in the workspace imports ONLY from this file:
//
//   use primitives::{Amount, BlockHeight, Timestamp, Nonce};
//   use primitives::constants::*;
//   use primitives::{block_reward_at, micro_to_display};
//   use primitives::PrimitivesError;

// ─── Internal modules ─────────────────────────────────────────────────────────

mod error;
mod types;
mod units;

// constants is pub — downstream crates need direct access to the values.
pub mod constants;

// ─── Public re-exports ────────────────────────────────────────────────────────

/// Unified error type for all primitives operations.
pub use error::PrimitivesError;

/// Non-negative monetary amount in micro-tokens.
pub use types::Amount;

/// Block height (chain index starting at 0).
pub use types::BlockHeight;

/// Unix timestamp in milliseconds.
pub use types::Timestamp;

/// Per-account transaction nonce (replay protection counter).
pub use types::Nonce;

/// Format micro-tokens as a decimal string: `micro_to_display(1_500_000)` → `"1.500000"`.
pub use units::micro_to_display;

/// Parse a decimal string into micro-tokens: `display_to_micro("1.5")` → `Ok(1_500_000)`.
pub use units::display_to_micro;

/// Block reward at a given height (halving-aware, consensus-critical).
pub use units::block_reward_at;

/// Check that an Amount does not exceed MAX_SUPPLY_MICRO.
pub use units::is_supply_valid;

/// Theoretical total supply minted up to (but not including) a given height.
pub use units::theoretical_supply_at;

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use constants::*;

    // ── Amount constructors ───────────────────────────────────────────────────

    #[test]
    fn test_amount_from_micro_valid() {
        let a = Amount::from_micro(1_000_000).unwrap();
        assert_eq!(a.as_micro(), 1_000_000);
    }

    #[test]
    fn test_amount_from_micro_exceeds_supply() {
        assert!(Amount::from_micro(MAX_SUPPLY_MICRO + 1).is_err());
    }

    #[test]
    fn test_amount_from_micro_max_supply_ok() {
        assert!(Amount::from_micro(MAX_SUPPLY_MICRO).is_ok());
    }

    #[test]
    fn test_amount_from_tokens_valid() {
        let a = Amount::from_tokens(50).unwrap();
        assert_eq!(a.as_micro(), 50 * MICRO_PER_TOKEN);
    }

    #[test]
    fn test_amount_from_tokens_overflow() {
        // u64::MAX × MICRO_PER_TOKEN overflows u64.
        assert!(Amount::from_tokens(u64::MAX).is_err());
    }

    #[test]
    fn test_amount_zero() {
        assert!(Amount::ZERO.is_zero());
        assert_eq!(Amount::ZERO.as_micro(), 0);
    }

    // ── Amount arithmetic ─────────────────────────────────────────────────────

    #[test]
    fn test_amount_checked_add_ok() {
        let a = Amount::from_micro(1_000).unwrap();
        let b = Amount::from_micro(2_000).unwrap();
        assert_eq!(a.checked_add(b).unwrap().as_micro(), 3_000);
    }

    #[test]
    fn test_amount_checked_add_overflow() {
        // Sum exceeds MAX_SUPPLY_MICRO.
        let a = Amount::from_micro(MAX_SUPPLY_MICRO).unwrap();
        let b = Amount::from_micro(1).unwrap();
        assert!(a.checked_add(b).is_none());
    }

    #[test]
    fn test_amount_checked_sub_ok() {
        let a = Amount::from_micro(5_000).unwrap();
        let b = Amount::from_micro(3_000).unwrap();
        assert_eq!(a.checked_sub(b).unwrap().as_micro(), 2_000);
    }

    #[test]
    fn test_amount_checked_sub_underflow() {
        let a = Amount::from_micro(1_000).unwrap();
        let b = Amount::from_micro(2_000).unwrap();
        assert!(a.checked_sub(b).is_none());
    }

    #[test]
    fn test_amount_scale() {
        let fee_per_byte = Amount::from_micro(10).unwrap();
        assert_eq!(fee_per_byte.scale(100).unwrap().as_micro(), 1_000);
    }

    #[test]
    fn test_amount_scale_overflow() {
        let a = Amount::from_micro(MAX_SUPPLY_MICRO).unwrap();
        assert!(a.scale(2).is_none());
    }

    // ── Amount display ────────────────────────────────────────────────────────

    #[test]
    fn test_amount_display_whole() {
        let a = Amount::from_tokens(1).unwrap();
        assert_eq!(a.to_display_string(), "1.000000");
    }

    #[test]
    fn test_amount_display_fractional() {
        let a = Amount::from_micro(1_234_567).unwrap();
        assert_eq!(a.to_display_string(), "1.234567");
    }

    #[test]
    fn test_amount_display_zero() {
        assert_eq!(Amount::ZERO.to_display_string(), "0.000000");
    }

    #[test]
    fn test_amount_to_tokens_parts() {
        let a = Amount::from_micro(2_500_000).unwrap();
        assert_eq!(a.to_tokens_parts(), (2, 500_000));
    }

    // ── BlockHeight ───────────────────────────────────────────────────────────

    #[test]
    fn test_block_height_genesis() {
        let h = BlockHeight::genesis();
        assert!(h.is_genesis());
        assert_eq!(h.as_u64(), 0);
    }

    #[test]
    fn test_block_height_next() {
        let h = BlockHeight::genesis().next();
        assert_eq!(h.as_u64(), 1);
        assert!(!h.is_genesis());
    }

    #[test]
    fn test_block_height_halving_epoch() {
        assert_eq!(BlockHeight::new(0).halving_epoch(), 0);
        assert_eq!(BlockHeight::new(HALVING_INTERVAL - 1).halving_epoch(), 0);
        assert_eq!(BlockHeight::new(HALVING_INTERVAL).halving_epoch(), 1);
        assert_eq!(BlockHeight::new(HALVING_INTERVAL * 2).halving_epoch(), 2);
    }

    #[test]
    fn test_block_height_is_halving_block() {
        assert!(!BlockHeight::new(0).is_halving_block());   // genesis is not
        assert!(!BlockHeight::new(1).is_halving_block());
        assert!(BlockHeight::new(HALVING_INTERVAL).is_halving_block());
        assert!(BlockHeight::new(HALVING_INTERVAL * 2).is_halving_block());
    }

    #[test]
    fn test_block_height_display() {
        assert_eq!(format!("{}", BlockHeight::new(42)), "#42");
    }

    // ── Timestamp ─────────────────────────────────────────────────────────────

    #[test]
    fn test_timestamp_ordering() {
        let t1 = Timestamp::from_millis(1_000);
        let t2 = Timestamp::from_millis(2_000);
        assert!(t2.is_after(&t1));
        assert!(t1.is_before(&t2));
        assert!(!t1.is_after(&t2));
    }

    #[test]
    fn test_timestamp_millis_since() {
        let t1 = Timestamp::from_millis(1_000);
        let t2 = Timestamp::from_millis(3_000);
        assert_eq!(t2.millis_since(&t1), Some(2_000));
        assert_eq!(t1.millis_since(&t2), None); // t1 < t2 → negative
    }

    #[test]
    fn test_timestamp_validate_not_future_ok() {
        let now    = Timestamp::from_millis(10_000);
        let future = Timestamp::from_millis(11_000); // 1s in future, within 2min
        assert!(future.validate_not_future(&now, MAX_FUTURE_BLOCK_TIME_MS).is_ok());
    }

    #[test]
    fn test_timestamp_validate_not_future_rejected() {
        let now    = Timestamp::from_millis(10_000);
        // 3 minutes in the future — exceeds MAX_FUTURE_BLOCK_TIME_MS.
        let future = Timestamp::from_millis(10_000 + 180_000);
        assert!(future.validate_not_future(&now, MAX_FUTURE_BLOCK_TIME_MS).is_err());
    }

    #[test]
    fn test_timestamp_now_is_reasonable() {
        let t = Timestamp::now();
        // Must be after 2024-01-01 00:00:00 UTC (ms = 1_704_067_200_000).
        assert!(t.as_millis() > 1_704_067_200_000);
    }

    // ── Nonce ─────────────────────────────────────────────────────────────────

    #[test]
    fn test_nonce_zero() {
        assert_eq!(Nonce::zero().as_u64(), 0);
    }

    #[test]
    fn test_nonce_next() {
        let n = Nonce::zero().next().unwrap();
        assert_eq!(n.as_u64(), 1);
    }

    #[test]
    fn test_nonce_follows() {
        let n0 = Nonce::new(0);
        let n1 = Nonce::new(1);
        let n2 = Nonce::new(2);
        assert!(n1.follows(&n0));  // 1 follows 0 ✓
        assert!(!n2.follows(&n0)); // 2 does not follow 0 ✗
        assert!(!n0.follows(&n0)); // 0 does not follow 0 ✗
    }

    #[test]
    fn test_nonce_overflow() {
        let max = Nonce::new(u64::MAX);
        assert!(max.next().is_err());
    }

    // ── micro_to_display / display_to_micro ───────────────────────────────────

    #[test]
    fn test_micro_to_display_roundtrip() {
        let cases: &[(&str, u64)] = &[
            ("0.000000",          0),
            ("1.000000",          1_000_000),
            ("1.500000",          1_500_000),
            ("0.000001",          1),
            ("21000000.000000",   21_000_000 * MICRO_PER_TOKEN),
        ];
        for (expected, micro) in cases {
            assert_eq!(micro_to_display(*micro), *expected, "micro={}", micro);
        }
    }

    #[test]
    fn test_display_to_micro_valid() {
        assert_eq!(display_to_micro("1").unwrap(), 1_000_000);
        assert_eq!(display_to_micro("1.5").unwrap(), 1_500_000);
        assert_eq!(display_to_micro("0.000001").unwrap(), 1);
        assert_eq!(display_to_micro("0").unwrap(), 0);
    }

    #[test]
    fn test_display_to_micro_too_many_decimals() {
        assert!(display_to_micro("1.1234567").is_err()); // 7 decimal places
    }

    #[test]
    fn test_display_to_micro_exceeds_supply() {
        assert!(display_to_micro("99999999999999").is_err());
    }

    #[test]
    fn test_display_to_micro_invalid_chars() {
        assert!(display_to_micro("abc").is_err());
        assert!(display_to_micro("1.2a").is_err());
    }

    // ── block_reward_at ───────────────────────────────────────────────────────

    #[test]
    fn test_block_reward_epoch_0() {
        // Heights 0 through HALVING_INTERVAL-1 → full reward.
        assert_eq!(
            block_reward_at(&BlockHeight::new(0)).as_micro(),
            INITIAL_BLOCK_REWARD
        );
        assert_eq!(
            block_reward_at(&BlockHeight::new(HALVING_INTERVAL - 1)).as_micro(),
            INITIAL_BLOCK_REWARD
        );
    }

    #[test]
    fn test_block_reward_epoch_1() {
        assert_eq!(
            block_reward_at(&BlockHeight::new(HALVING_INTERVAL)).as_micro(),
            INITIAL_BLOCK_REWARD / 2
        );
    }

    #[test]
    fn test_block_reward_epoch_2() {
        assert_eq!(
            block_reward_at(&BlockHeight::new(HALVING_INTERVAL * 2)).as_micro(),
            INITIAL_BLOCK_REWARD / 4
        );
    }

    #[test]
    fn test_block_reward_after_max_halvings() {
        // After MAX_HALVINGS the reward is 0 (fees only).
        let far_height = BlockHeight::new(HALVING_INTERVAL * (MAX_HALVINGS as u64 + 10));
        assert_eq!(block_reward_at(&far_height), Amount::ZERO);
    }

    #[test]
    fn test_block_reward_is_deterministic() {
        // Same height always returns same reward.
        let h = BlockHeight::new(12_345);
        assert_eq!(block_reward_at(&h), block_reward_at(&h));
    }

    // ── theoretical_supply_at ─────────────────────────────────────────────────

    #[test]
    fn test_theoretical_supply_at_genesis() {
        // Before any block is mined, supply is 0.
        let s = theoretical_supply_at(&BlockHeight::genesis()).unwrap();
        assert_eq!(s, Amount::ZERO);
    }

    #[test]
    fn test_theoretical_supply_at_block_1() {
        // First block mints INITIAL_BLOCK_REWARD.
        let s = theoretical_supply_at(&BlockHeight::new(1)).unwrap();
        assert_eq!(s.as_micro(), INITIAL_BLOCK_REWARD);
    }

    #[test]
    fn test_theoretical_supply_never_exceeds_max() {
        // At a very large height, supply must not exceed MAX_SUPPLY_MICRO.
        let h = BlockHeight::new(u64::MAX / HALVING_INTERVAL * HALVING_INTERVAL);
        if let Some(s) = theoretical_supply_at(&h) {
            assert!(s.as_micro() <= MAX_SUPPLY_MICRO);
        }
    }

    // ── is_supply_valid ───────────────────────────────────────────────────────

    #[test]
    fn test_is_supply_valid() {
        assert!(is_supply_valid(&Amount::ZERO));
        assert!(is_supply_valid(&Amount::MAX));
        // MAX+1 cannot be constructed via from_micro (it would error),
        // so we test the boundary through from_micro_unchecked indirectly
        // by checking MAX is valid.
        assert!(is_supply_valid(&Amount::from_micro(MAX_SUPPLY_MICRO).unwrap()));
    }
}
