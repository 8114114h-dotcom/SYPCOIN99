// executor.rs — Unified Executor facade.
//
// Executor wraps WorldState and provides a single entry point for the
// node layer to execute blocks and transactions without directly touching
// the state internals.

use block::Block;
use crypto::Address;
use primitives::Timestamp;
use state::{WorldState, StateSnapshot};
use transaction::Transaction;

use crate::block_executor::{BlockExecutor, BlockReceipt};
use crate::error::ExecutionError;
use crate::tx_executor::{TxExecutor, TxReceipt};

/// The main execution engine.
///
/// Owns a `WorldState` and exposes methods to execute blocks and transactions.
/// The node layer holds one `Executor` instance for the canonical chain.
pub struct Executor {
    state: WorldState,
}

impl Executor {
    /// Create a new executor wrapping an existing world state.
    pub fn new(state: WorldState) -> Self {
        Executor { state }
    }

    // ── Block execution ───────────────────────────────────────────────────────

    /// Execute a full block and commit the resulting state.
    pub fn execute_block(&mut self, block: &Block) -> Result<BlockReceipt, ExecutionError> {
        BlockExecutor::execute(&mut self.state, block)
    }

    /// Simulate block execution without modifying state.
    ///
    /// Useful for validating a block template before broadcasting.
    pub fn dry_run_block(&self, block: &Block) -> Result<BlockReceipt, ExecutionError> {
        BlockExecutor::dry_run(&self.state, block)
    }

    // ── Transaction execution ─────────────────────────────────────────────────

    /// Execute a single transaction against the current state.
    ///
    /// Used for testing individual transactions outside a block context.
    pub fn execute_tx(
        &mut self,
        tx:    &Transaction,
        miner: &Address,
        now:   Timestamp,
    ) -> Result<TxReceipt, ExecutionError> {
        TxExecutor::execute(&mut self.state, tx, miner, now)
    }

    // ── State access ──────────────────────────────────────────────────────────

    /// Read-only reference to the world state.
    pub fn state(&self) -> &WorldState {
        &self.state
    }

    /// Consume the executor and return the owned world state.
    pub fn into_state(self) -> WorldState {
        self.state
    }

    /// Take a snapshot of the current state.
    pub fn snapshot(&self) -> StateSnapshot {
        self.state.snapshot()
    }

    /// Restore state from a snapshot (e.g. after a failed reorg).
    pub fn restore(&mut self, snapshot: StateSnapshot) {
        self.state.restore_from_snapshot(snapshot);
    }
}
