// pow/mtp.rs — Median Time Past (MTP) implementation.
//
// Purpose:
//   Prevent timestamp manipulation attacks where a miner backdates
//   a block's timestamp to reduce difficulty or enable double-spend
//   by reversing the apparent order of transactions.
//
// Algorithm (same as Bitcoin):
//   MTP(block N) = median timestamp of blocks [N-11 .. N-1]
//
//   Rule: block N's timestamp MUST be strictly greater than MTP(N).
//
// Why median and not mean?
//   A single miner cannot move the median by more than one position
//   per block mined. To shift MTP by k seconds, an attacker must
//   control at least 6 consecutive blocks (majority of the 11-window).
//   This is the same security assumption as the 51% threshold.
//
// Why 11 blocks?
//   Bitcoin's choice — long enough to smooth variance, short enough
//   that the timestamp is still representative of recent time.

use primitives::Timestamp;

/// Number of historical blocks used for MTP calculation.
pub const MTP_WINDOW: usize = 11;

/// Compute the Median Time Past from a window of recent timestamps.
///
/// `recent_timestamps` should contain the last `MTP_WINDOW` block
/// timestamps, ordered oldest-first. If fewer blocks are available
/// (e.g. near genesis), uses whatever is provided.
///
/// Returns `None` if the slice is empty.
pub fn median_time_past(recent_timestamps: &[Timestamp]) -> Option<Timestamp> {
    if recent_timestamps.is_empty() {
        return None;
    }

    let mut sorted: Vec<u64> = recent_timestamps
        .iter()
        .map(|t| t.as_millis())
        .collect();

    sorted.sort_unstable();

    let mid = sorted.len() / 2;
    Some(Timestamp::from_millis(sorted[mid]))
}

/// Validate that a new block's timestamp is strictly after the MTP.
///
/// Returns `Err` with a descriptive message if the timestamp is invalid.
pub fn validate_mtp(
    block_timestamp:     Timestamp,
    recent_timestamps:   &[Timestamp],
) -> Result<(), String> {
    let mtp = match median_time_past(recent_timestamps) {
        Some(t) => t,
        None    => return Ok(()), // genesis or near-genesis — no MTP constraint
    };

    if block_timestamp.as_millis() <= mtp.as_millis() {
        return Err(format!(
            "block timestamp {} ms is not after MTP {} ms — possible timestamp manipulation",
            block_timestamp.as_millis(),
            mtp.as_millis()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(ms: u64) -> Timestamp { Timestamp::from_millis(ms) }

    #[test]
    fn mtp_is_median_of_window() {
        let timestamps = vec![
            ts(100), ts(200), ts(150), ts(300), ts(250),
            ts(400), ts(350), ts(500), ts(450), ts(600), ts(550),
        ];
        // sorted: 100,150,200,250,300,350,400,450,500,550,600
        // median (index 5): 350
        assert_eq!(median_time_past(&timestamps).unwrap().as_millis(), 350);
    }

    #[test]
    fn mtp_rejects_backdated_block() {
        let window: Vec<Timestamp> = (1..=11).map(|i| ts(i * 1000)).collect();
        // MTP = median of 1000..11000 = 6000
        // Block at 5000 ms should be rejected
        assert!(validate_mtp(ts(5000), &window).is_err());
        // Block at 7000 ms should pass
        assert!(validate_mtp(ts(7000), &window).is_ok());
    }

    #[test]
    fn mtp_allows_equal_to_median_to_fail() {
        let window: Vec<Timestamp> = (1..=11).map(|i| ts(i * 1000)).collect();
        // Exactly equal to MTP is also rejected (must be STRICTLY greater)
        assert!(validate_mtp(ts(6000), &window).is_err());
    }

    #[test]
    fn empty_window_always_passes() {
        assert!(validate_mtp(ts(0), &[]).is_ok());
    }
}
