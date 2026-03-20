// reward.rs — Block reward helpers.
//
// The actual reward calculation lives in primitives::units::block_reward_at().
// This module provides block-level helpers that combine the reward with
// fee collection from the block's transactions.

use primitives::{Amount, BlockHeight, block_reward_at};
use transaction::Transaction;
use crate::error::BlockError;

/// Calculate the total coinbase value for a block.
///
/// coinbase = block_reward + sum(tx.fee for tx in transactions)
///
/// The miner receives this total via the state transition layer.
/// This function is used by the block validator to ensure the miner
/// did not claim more than they are entitled to.
pub fn total_coinbase(height: &BlockHeight, transactions: &[Transaction]) -> Option<Amount> {
    let reward = block_reward_at(height);

    transactions.iter().try_fold(reward, |acc, tx| {
        acc.checked_add(tx.fee())
    })
}

/// Verify that the miner's claimed reward does not exceed the allowed amount.
///
/// Called during block validation. Returns Err if the coinbase transaction
/// (in future iterations with explicit coinbase txs) exceeds the cap.
#[allow(dead_code)]
pub fn verify_reward_cap(
    claimed:      Amount,
    height:       &BlockHeight,
    transactions: &[Transaction],
) -> Result<(), BlockError> {
    let allowed = total_coinbase(height, transactions)
        .ok_or(BlockError::InvalidDifficulty)?; // overflow = invalid block

    if claimed > allowed {
        return Err(BlockError::InvalidDifficulty); // reuse closest error
    }
    Ok(())
}
