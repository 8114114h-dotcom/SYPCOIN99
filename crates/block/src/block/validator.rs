// block/validator.rs — Block validation.
//
// Three levels (cheapest → most expensive):
//
//   validate_structure()      — pure, no external state, O(1)
//     1. Version check        — reject unknown versions first
//     2. Size limits          — reject oversized blocks before hashing
//     3. Timestamp future     — reject far-future timestamps
//     4. tx_count consistency — header vs body
//     5. Merkle root          — integrity check (O(n) hashing)
//
//   validate_against_parent() — requires the parent BlockHeader
//     1. Height sequence      — must be parent + 1
//     2. Parent hash linkage  — cryptographic chain link
//     3. Timestamp monotone   — must be after parent (prevents reordering)
//
//   validate_pow()            — expensive hash comparison
//     NOTE: Called last — only after structure and parent checks pass.
//     This prevents an attacker from forcing expensive PoW checks on
//     structurally invalid blocks.
//
// SECURITY NOTE: MTP (Median Time Past) validation is performed in
// chain_rules.rs using the last 11 block timestamps. It cannot be
// done here because validate_structure() has no access to history.

use primitives::constants::MAX_FUTURE_BLOCK_TIME_MS;
use primitives::Timestamp;

use crate::block::block::Block;
use crate::block_hash::{difficulty_to_target, meets_target};
use crate::error::BlockError;
use crate::header::header::{BlockHeader, BLOCK_VERSION};
use crate::size_limit::{check_block_size, check_tx_count};

pub struct BlockValidator;

impl BlockValidator {
    /// Structural validation — no state required.
    ///
    /// Checks: version, difficulty, tx_count consistency,
    /// merkle_root integrity, block size, timestamp future drift.
    pub fn validate_structure(block: &Block) -> Result<(), BlockError> {
        let h = block.header();

        // 1. Version.
        if h.version() != BLOCK_VERSION {
            return Err(BlockError::InvalidVersion(h.version()));
        }

        // 2. Difficulty must be non-zero.
        if h.difficulty() == 0 {
            return Err(BlockError::InvalidDifficulty);
        }

        // 3. tx_count in header must match body.
        if h.tx_count() != block.body().tx_count() {
            return Err(BlockError::TxCountMismatch {
                header: h.tx_count(),
                body:   block.body().tx_count(),
            });
        }

        // 4. Transaction count within limits.
        check_tx_count(block.tx_count())?;

        // 5. Merkle root must match computed root.
        let computed = block.body().merkle_root();
        if computed.as_bytes() != h.merkle_root().as_bytes() {
            return Err(BlockError::InvalidMerkleRoot);
        }

        // 6. Block size within limits.
        check_block_size(block.size_bytes())?;

        // 7. Timestamp must not be too far in the future.
        let now = Timestamp::now();
        block.timestamp()
            .validate_not_future(&now, MAX_FUTURE_BLOCK_TIME_MS)
            .map_err(|_| BlockError::InvalidTimestamp(
                "timestamp too far in the future".into()
            ))?;

        Ok(())
    }

    /// Validate this block against its parent header.
    ///
    /// Checks: height sequence, parent hash linkage, timestamp monotonicity.
    pub fn validate_against_parent(
        block:  &Block,
        parent: &BlockHeader,
    ) -> Result<(), BlockError> {
        // 1. Height must be parent.height + 1.
        let expected_height = parent.height().next().as_u64();
        if block.height().as_u64() != expected_height {
            return Err(BlockError::InvalidHeight {
                expected: expected_height,
                got:      block.height().as_u64(),
            });
        }

        // 2. parent_hash must match the actual parent's hash.
        let actual_parent_hash = parent.hash();
        if block.parent_hash().as_bytes() != actual_parent_hash.as_bytes() {
            return Err(BlockError::InvalidParentHash);
        }

        // 3. Timestamp must be strictly after parent timestamp.
        //    Allow small drift but must be > parent.
        if !block.timestamp().is_after(&parent.timestamp()) {
            return Err(BlockError::InvalidTimestamp(
                "block timestamp must be after parent timestamp".into()
            ));
        }

        Ok(())
    }

    /// Validate Proof-of-Work.
    ///
    /// The block hash (as a 256-bit big-endian integer) must be less than
    /// the target derived from the block's difficulty field.
    pub fn validate_pow(block: &Block) -> Result<(), BlockError> {
        let hash   = block.hash();
        let target = difficulty_to_target(block.difficulty());

        if !meets_target(&hash, &target) {
            return Err(BlockError::InsufficientPoW {
                hash:   hex::encode(hash.as_bytes()),
                target: hex::encode(&target),
            });
        }

        Ok(())
    }
}
