// size_limit.rs — Block size limits.
//
// Consensus-critical: every node must apply the same size limits.
// A block exceeding any limit is rejected regardless of PoW validity.

use primitives::constants::{MAX_BLOCK_SIZE, MAX_TX_PER_BLOCK};
use crate::error::BlockError;

/// Validate that a block's transaction count is within limits.
pub fn check_tx_count(count: u32) -> Result<(), BlockError> {
    if count > MAX_TX_PER_BLOCK {
        return Err(BlockError::TooManyTransactions {
            max: MAX_TX_PER_BLOCK,
            got: count,
        });
    }
    Ok(())
}

/// Validate that a block's serialized size is within limits.
pub fn check_block_size(size_bytes: usize) -> Result<(), BlockError> {
    if size_bytes > MAX_BLOCK_SIZE as usize {
        return Err(BlockError::BlockTooLarge {
            max: MAX_BLOCK_SIZE as usize,
            got: size_bytes,
        });
    }
    Ok(())
}
