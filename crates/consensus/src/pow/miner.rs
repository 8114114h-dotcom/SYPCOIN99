// pow/miner.rs — Proof-of-Work mining loop.
//
// The miner iterates over nonce values (0..u64::MAX) and checks whether
// SHA-256(SHA-256(DOMAIN || header_bytes)) < target(difficulty).
//
// Design decisions:
//
//   INCREMENTAL NONCE
//   • We start at nonce=0 and increment by 1. In production a miner would
//     use multiple threads each covering a different nonce range. The single-
//     threaded loop here is correct; parallelism is an optimisation left to
//     the node layer.
//
//   BLOCK TEMPLATE
//   • The miner receives a complete Block (built by BlockBuilder with nonce=0)
//     and mutates only the nonce field. All other fields (merkle_root,
//     state_root, transactions) are fixed before mining starts.
//
//   EARLY EXIT
//   • mine() accepts an optional callback `should_stop: &dyn Fn() -> bool`
//     so the node can cancel mining when a new block arrives from the network.
//
//   MAX_NONCE
//   • If u64::MAX nonces are exhausted without a solution (astronomically
//     unlikely at any reasonable difficulty), mining fails. In practice the
//     timestamp is updated before this can happen (extra-nonce technique).

use block::{Block, difficulty_to_target, meets_target};
use primitives::Timestamp;

use crate::error::ConsensusError;

/// Result of a successful mining operation.
#[derive(Debug)]
pub struct MineResult {
    /// The solved block with the valid nonce.
    pub block:       Block,
    /// The winning nonce value.
    pub nonce_found: u64,
    /// Number of nonces attempted.
    pub nonces_tried: u64,
    /// Wall-clock milliseconds elapsed.
    pub elapsed_ms:  u64,
}

/// Proof-of-Work miner.
pub struct Miner {
    /// The miner's reward address.
    pub address: crypto::Address,
}

impl Miner {
    pub fn new(address: crypto::Address) -> Self {
        Miner { address }
    }

    /// Attempt to mine a block by finding a nonce that satisfies the PoW target.
    ///
    /// # Arguments
    /// - `template`     — A complete block with nonce=0. All other fields fixed.
    /// - `should_stop`  — Called every 1000 iterations; returns `true` to abort.
    ///
    /// # Returns
    /// `Ok(MineResult)` if a valid nonce was found.
    /// `Err(MiningFailed)` if all nonces exhausted or `should_stop` returned true.
    pub fn mine<F>(
        &self,
        mut template: Block,
        should_stop:  F,
    ) -> Result<MineResult, ConsensusError>
    where
        F: Fn() -> bool,
    {
        let target      = difficulty_to_target(template.difficulty());
        let start_ms    = Timestamp::now().as_millis();
        let mut nonce   = 0u64;

        loop {
            // Update the nonce in the block header.
            template.set_nonce(nonce);

            // Check if this nonce produces a valid hash.
            let hash = template.hash();
            if meets_target(&hash, &target) {
                let elapsed_ms = Timestamp::now()
                    .as_millis()
                    .saturating_sub(start_ms);
                return Ok(MineResult {
                    block:        template,
                    nonce_found:  nonce,
                    nonces_tried: nonce + 1,
                    elapsed_ms,
                });
            }

            // Check cancellation every 10_000 iterations.
            if nonce % 10_000 == 0 && should_stop() {
                return Err(ConsensusError::MiningFailed);
            }

            nonce = match nonce.checked_add(1) {
                Some(n) => n,
                None    => return Err(ConsensusError::MiningFailed),
            };
        }
    }
}
