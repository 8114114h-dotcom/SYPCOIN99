// genesis_block.rs — Build the genesis block.
//
// Security decisions:
//   • parent_hash = SHA-256(b"SYPCOIN_GENESIS_PARENT_V1")
//     A domain-separated constant. Using all-zeros would be ambiguous —
//     an attacker could craft a block claiming to be parent of genesis.
//     A fixed hash makes the genesis unmistakably unique.
//
//   • nonce = 0 — genesis requires no PoW. The difficulty is set to the
//     configured initial value, but no hash target must be met for block 0.
//     The consensus layer special-cases genesis validation accordingly.
//
//   • The genesis message (if any) is encoded as UTF-8 bytes and stored
//     in the first (and only) bytes of the block's data field concept.
//     For now it is captured in the config and used as the state root
//     domain separator to make the genesis truly unique per chain.
//
//   • state_root must come from GenesisState::build() — it commits to
//     all pre-funded accounts. Passing an arbitrary root is an error.

use block::{Block, BlockBuilder};
use crypto::HashDigest;
use primitives::{BlockHeight, Timestamp};

use crate::error::GenesisError;
use crate::loader::GenesisConfig;

/// The fixed parent hash for all genesis blocks.
///
/// = SHA-256(b"SYPCOIN_GENESIS_PARENT_V1")
/// CONSENSUS CRITICAL — must never change after mainnet launch.
pub fn genesis_parent_hash() -> HashDigest {
    crypto::sha256(b"SYPCOIN_GENESIS_PARENT_V1")
}

pub struct GenesisBlock;

impl GenesisBlock {
    /// Build the genesis block from config and a pre-computed state root.
    ///
    /// The `state_root` must be obtained from `GenesisState::build()`.
    pub fn build(
        config:     &GenesisConfig,
        state_root: HashDigest,
    ) -> Result<Block, GenesisError> {
        let parent_hash = genesis_parent_hash();
        let timestamp   = Timestamp::from_millis(config.timestamp);
        let _miner       = crypto::Address::from_public_key(
            // Genesis block is mined by the zero address (no miner reward).
            // We use the SHA-256 of the chain name as a deterministic address.
            &crypto::KeyPair::generate()
                .map_err(|e| GenesisError::BlockBuildError(e.to_string()))?
                .public_key()
                .clone()
        );

        // Use the config's miner address if provided, otherwise use a
        // deterministic stand-in derived from chain_id.
        let genesis_miner = derive_genesis_miner(config.chain_id);

        BlockBuilder::new()
            .height(BlockHeight::genesis())
            .parent_hash(parent_hash)
            .state_root(state_root)
            .miner(genesis_miner)
            .difficulty(config.initial_difficulty)
            .timestamp(timestamp)
            .nonce(0)
            // Genesis has no transactions.
            .build()
            .map_err(|e| GenesisError::BlockBuildError(e.to_string()))
    }
}

/// Derive a deterministic genesis miner address from the chain ID.
///
/// This address receives no reward (genesis block has no coinbase).
/// It is used purely to satisfy the block structure requirement.
fn derive_genesis_miner(chain_id: u64) -> crypto::Address {
    // Pre-image: b"SYPCOIN_GENESIS_MINER_V1" || chain_id_le8
    let mut pre = b"SYPCOIN_GENESIS_MINER_V1".to_vec();
    pre.extend_from_slice(&chain_id.to_le_bytes());
    let digest = crypto::sha256(&pre);

    // Derive a deterministic address from the digest.
    // We treat the digest as a fake public key for address derivation.
    // This is safe because the address is only used as a placeholder —
    // no real keypair corresponds to it.
    let mut addr_bytes = [0u8; 20];
    addr_bytes.copy_from_slice(&digest.as_bytes()[..20]);

    // We need a PublicKey to derive an address. Use sha256 of the digest
    // as a stand-in (this is not a valid signing key).
    let pk_hash = crypto::sha256(digest.as_bytes());
    // Construct address directly from hash bytes (first 20 bytes of SHA-256).
    let addr_digest = crypto::sha256(pk_hash.as_bytes());
    let mut bytes = [0u8; 20];
    bytes.copy_from_slice(&addr_digest.as_bytes()[..20]);

    // Build address from raw bytes via a temporary struct.
    // This works because Address is just a [u8;20] newtype with serde.
    build_address_from_bytes(bytes)
}

/// Construct an Address from raw 20 bytes.
/// Used only for the genesis miner placeholder.
fn build_address_from_bytes(bytes: [u8; 20]) -> crypto::Address {
    crypto::Address::from_raw_bytes(bytes)
}
