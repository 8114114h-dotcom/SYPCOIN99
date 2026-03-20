// block_executor.rs — Full block execution engine.
//
// BlockExecutor applies an entire block to the world state:
//   1. Verify the block's height follows the current state.
//   2. Execute each transaction via TxExecutor.
//   3. Apply the block reward to the miner.
//   4. Commit: compute and store the new state_root.
//   5. Return a BlockReceipt summarising the outcome.
//
// dry_run():
//   Clones the state, runs the full execution, then discards the clone.
//   Used by the block proposer to verify a block template before committing.
//   Expensive (full state clone) but correct.
//
// Atomicity:
//   If apply_block_reward() fails, we restore the pre-block snapshot.
//   Individual tx failures are non-fatal — they produce Failed receipts.

use serde::{Deserialize, Serialize};

use block::Block;
use crypto::HashDigest;
use primitives::{Amount, BlockHeight, Timestamp};
use state::WorldState;

use crate::error::ExecutionError;
use crate::tx_executor::{TxExecutor, TxReceipt};

/// Summary of executing an entire block.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockReceipt {
    /// Hash of the block that was executed.
    pub block_hash:    HashDigest,
    /// Height of the block.
    pub block_height:  BlockHeight,
    /// Per-transaction execution results.
    pub tx_receipts:   Vec<TxReceipt>,
    /// Sum of all fees collected in this block.
    pub total_fees:    Amount,
    /// Block reward paid to the miner.
    pub reward_paid:   Amount,
    /// State root after applying this block.
    pub state_root:    HashDigest,
    /// Number of transactions that succeeded.
    pub txs_succeeded: u32,
    /// Number of transactions that failed.
    pub txs_failed:    u32,
}

pub struct BlockExecutor;

impl BlockExecutor {
    /// Execute a block against the world state and commit.
    ///
    /// On success, the state is updated and a `BlockReceipt` is returned.
    /// On hard failure (reward application error), the state is rolled back
    /// to a snapshot taken before execution began.
    pub fn execute(
        state: &mut WorldState,
        block: &Block,
    ) -> Result<BlockReceipt, ExecutionError> {
        // ── 1. Height check ───────────────────────────────────────────────────
        let expected_height = state.block_height().next().as_u64();
        let got_height      = block.height().as_u64();

        // Genesis special case: if state is at genesis (0) and block is also 0.
        if !state.block_height().is_genesis() || !block.is_genesis() {
            if got_height != expected_height {
                return Err(ExecutionError::BlockHeightMismatch {
                    expected: expected_height,
                    got:      got_height,
                });
            }
        }

        // ── 2. Snapshot before execution (for rollback on hard failure) ────────
        let pre_snapshot = state.snapshot();
        let now          = Timestamp::now();
        let miner        = block.miner().clone();

        // ── 3. Execute transactions ───────────────────────────────────────────
        let mut tx_receipts   = Vec::with_capacity(block.tx_count() as usize);
        let mut total_fees    = Amount::ZERO;
        let mut txs_succeeded = 0u32;
        let mut txs_failed    = 0u32;

        for tx in block.transactions() {
            let receipt = TxExecutor::execute(state, tx, &miner, now)?;

            if receipt.status.is_success() {
                total_fees = total_fees
                    .checked_add(receipt.fee)
                    .unwrap_or(total_fees);
                txs_succeeded += 1;
            } else {
                txs_failed += 1;
            }

            tx_receipts.push(receipt);
        }

        // ── 4. Apply block reward ─────────────────────────────────────────────
        let reward_paid = state
            .apply_block_reward(&miner, &block.height())
            .map_err(|e| {
                // Hard failure: roll back to pre-execution state.
                state.restore_from_snapshot(pre_snapshot.clone());
                ExecutionError::RewardFailed(e.to_string())
            })?;

        // ── 5. Commit — computes new state_root ───────────────────────────────
        let state_root = state.commit(block.height());

        Ok(BlockReceipt {
            block_hash: block.hash(),
            block_height: block.height(),
            tx_receipts,
            total_fees,
            reward_paid,
            state_root,
            txs_succeeded,
            txs_failed,
        })
    }

    /// Execute a block without modifying the real state (simulation).
    ///
    /// Clones the state, runs full execution, discards the clone.
    /// Used by the block proposer to pre-validate a block template.
    pub fn dry_run(
        state: &WorldState,
        block: &Block,
    ) -> Result<BlockReceipt, ExecutionError> {
        // Clone is O(n accounts) — acceptable for block proposal.
        // In production, this would use a copy-on-write overlay.
        let snapshot = state.snapshot();
        let mut temp_state = WorldState::new();
        temp_state.restore_from_snapshot(snapshot);
        Self::execute(&mut temp_state, block)
    }
}
