// examples/usage.rs — End-to-end demo of the block crate.
//
// Run with:  cargo run --example usage

use crypto::{Address, KeyPair, sha256};
use primitives::{Amount, BlockHeight, Nonce, Timestamp};
use primitives::constants::{MIN_TX_FEE_MICRO, INITIAL_BLOCK_REWARD};
use block::{BlockBuilder, BlockValidator, compute_merkle_root, total_coinbase};
use transaction::TransactionBuilder;

fn main() {
    println!("═══════════════════════════════════════════════════════");
    println!("  Block Layer — Example Usage");
    println!("═══════════════════════════════════════════════════════\n");

    let miner_kp   = KeyPair::generate().unwrap();
    let miner_addr = Address::from_public_key(miner_kp.public_key());

    // ── 1. Genesis block ──────────────────────────────────────────────────────
    println!("── 1. Genesis Block ────────────────────────────────────");
    let genesis = BlockBuilder::new()
        .height(BlockHeight::new(0))
        .parent_hash(sha256(b"genesis_parent"))
        .state_root(sha256(b"genesis_state"))
        .miner(miner_addr.clone())
        .difficulty(1)
        .timestamp(Timestamp::from_millis(1_700_000_000_000))
        .build()
        .unwrap();

    println!("  {}", genesis);
    println!("  Hash       : {}", hex::encode(genesis.hash().as_bytes()));
    println!("  Is genesis : {}", genesis.is_genesis());
    println!("  Size       : {} bytes", genesis.size_bytes());
    println!();

    // ── 2. Block with transactions ────────────────────────────────────────────
    println!("── 2. Block With Transactions ──────────────────────────");

    let txs: Vec<_> = (1..=3).map(|i| {
        let kp = KeyPair::generate().unwrap();
        TransactionBuilder::new()
            .from_keypair(kp)
            .to(miner_addr.clone())
            .amount(Amount::from_tokens(i).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(i))
            .build()
            .unwrap()
    }).collect();

    let merkle = compute_merkle_root(&txs);
    println!("  Merkle root : {}", hex::encode(merkle.as_bytes()));

    let block1 = BlockBuilder::new()
        .height(BlockHeight::new(1))
        .parent_hash(genesis.hash())
        .state_root(sha256(b"state_after_block_1"))
        .miner(miner_addr.clone())
        .difficulty(1)
        .timestamp(Timestamp::from_millis(1_700_000_010_000))
        .transactions(txs.clone())
        .build()
        .unwrap();

    println!("  {}", block1);
    println!("  Tx count   : {}", block1.tx_count());
    println!("  Size       : {} bytes", block1.size_bytes());
    println!();

    // ── 3. Structural validation ──────────────────────────────────────────────
    println!("── 3. Validation ───────────────────────────────────────");
    match BlockValidator::validate_structure(&block1) {
        Ok(())  => println!("  [✓] Structure valid"),
        Err(e)  => println!("  [✗] {}", e),
    }
    match BlockValidator::validate_against_parent(&block1, genesis.header()) {
        Ok(())  => println!("  [✓] Parent linkage valid"),
        Err(e)  => println!("  [✗] {}", e),
    }
    println!();

    // ── 4. Coinbase reward ────────────────────────────────────────────────────
    println!("── 4. Coinbase Reward ──────────────────────────────────");
    let height = BlockHeight::new(1);
    let total  = total_coinbase(&height, &txs).unwrap();
    let fees: u64 = txs.iter().map(|t| t.fee().as_micro()).sum();
    println!("  Block reward : {} tokens", primitives::micro_to_display(INITIAL_BLOCK_REWARD));
    println!("  Total fees   : {} tokens", primitives::micro_to_display(fees));
    println!("  Total coinbase: {}", total);
    println!();

    println!("═══════════════════════════════════════════════════════");
    println!("  All checks passed.");
    println!("═══════════════════════════════════════════════════════");
}
