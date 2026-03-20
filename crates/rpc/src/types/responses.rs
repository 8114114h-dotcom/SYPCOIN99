// types/responses.rs — Domain response structs serialized to JSON.

use serde::Serialize;

use block::Block;
use primitives::micro_to_display;
use storage::TxRecord;

/// Block summary returned by getBlock / getBlockByHeight.
#[derive(Serialize)]
pub struct BlockResponse {
    pub hash:        String,
    pub height:      u64,
    pub parent_hash: String,
    pub merkle_root: String,
    pub state_root:  String,
    pub timestamp:   u64,
    pub difficulty:  u64,
    pub nonce:       u64,
    pub miner:       String,
    pub tx_count:    u32,
    pub size_bytes:  usize,
}

impl BlockResponse {
    pub fn from_block(block: &Block) -> Self {
        BlockResponse {
            hash:        hex::encode(block.hash().as_bytes()),
            height:      block.height().as_u64(),
            parent_hash: hex::encode(block.parent_hash().as_bytes()),
            merkle_root: hex::encode(block.merkle_root().as_bytes()),
            state_root:  hex::encode(block.state_root().as_bytes()),
            timestamp:   block.timestamp().as_millis(),
            difficulty:  block.difficulty(),
            nonce:       block.nonce(),
            miner:       block.miner().to_checksum_hex(),
            tx_count:    block.tx_count(),
            size_bytes:  block.size_bytes(),
        }
    }
}

/// Transaction response returned by getTransaction.
#[derive(Serialize)]
pub struct TxResponse {
    pub tx_id:        String,
    pub from:         String,
    pub to:           String,
    pub amount:       String,   // decimal display e.g. "10.000000"
    pub fee:          String,
    pub nonce:        u64,
    pub timestamp:    u64,
    pub block_height: Option<u64>,
    pub block_hash:   Option<String>,
}

impl TxResponse {
    pub fn from_record(record: &TxRecord) -> Self {
        let tx = &record.tx;
        TxResponse {
            tx_id:        hex::encode(tx.tx_id().as_bytes()),
            from:         tx.from().to_checksum_hex(),
            to:           tx.to().to_checksum_hex(),
            amount:       micro_to_display(tx.amount().as_micro()),
            fee:          micro_to_display(tx.fee().as_micro()),
            nonce:        tx.nonce().as_u64(),
            timestamp:    tx.timestamp().as_millis(),
            block_height: Some(record.block_height.as_u64()),
            block_hash:   Some(hex::encode(record.block_hash.as_bytes())),
        }
    }
}

/// Mining information returned by getMiningInfo.
#[derive(Serialize)]
pub struct MiningInfo {
    pub height:     u64,
    pub difficulty: u64,
    pub best_hash:  String,
    pub target:     String,
}
