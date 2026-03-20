// examples/usage.rs — End-to-end demonstration of the transaction crate.
//
// Run with:
//   cargo run --example usage

use crypto::KeyPair;
use primitives::{Amount, BlockHeight, Nonce, Timestamp};
use primitives::constants::MIN_TX_FEE_MICRO;
use transaction::{
    Mempool, MempoolConfig, ReceiptStatus, TransactionBuilder,
    TransactionReceipt, TransactionValidator,
};

fn main() {
    println!("═══════════════════════════════════════════════════════");
    println!("  Transaction Layer — Example Usage");
    println!("═══════════════════════════════════════════════════════\n");

    // ── 1. Generate sender and recipient ─────────────────────────────────────
    println!("── 1. Key Generation ───────────────────────────────────");
    let sender_kp   = KeyPair::generate().unwrap();
    let recipient_kp = KeyPair::generate().unwrap();
    let sender_addr  = crypto::Address::from_public_key(sender_kp.public_key());
    let recipient_addr = crypto::Address::from_public_key(recipient_kp.public_key());

    println!("  Sender    : {}", sender_addr.to_checksum_hex());
    println!("  Recipient : {}", recipient_addr.to_checksum_hex());
    println!();

    // ── 2. Build a transaction ────────────────────────────────────────────────
    println!("── 2. Build Transaction ────────────────────────────────");
    let amount = Amount::from_tokens(10).unwrap();
    let fee    = Amount::from_micro(MIN_TX_FEE_MICRO).unwrap();

    let tx = TransactionBuilder::new()
        .from_keypair(sender_kp)
        .to(recipient_addr.clone())
        .amount(amount)
        .fee(fee)
        .nonce(Nonce::new(1))
        .data(b"hello blockchain".to_vec())
        .build()
        .unwrap();

    println!("  {}", tx);
    println!("  tx_id     : {}", hex::encode(tx.tx_id().as_bytes()));
    println!("  amount    : {}", tx.amount());
    println!("  fee       : {}", tx.fee());
    println!("  deducted  : {}", tx.total_deducted().unwrap());
    println!("  nonce     : {}", tx.nonce());
    println!("  data      : {:?}", std::str::from_utf8(tx.data().unwrap()).unwrap());
    println!("  size      : {} bytes", tx.size_bytes());
    println!();

    // ── 3. Structural validation ──────────────────────────────────────────────
    println!("── 3. Structural Validation ────────────────────────────");
    match TransactionValidator::validate_structure(&tx) {
        Ok(())  => println!("  [✓] Structure valid"),
        Err(e)  => println!("  [✗] {}", e),
    }

    // ── 4. State validation ───────────────────────────────────────────────────
    println!("\n── 4. State Validation ─────────────────────────────────");

    // Valid case: balance=100, account nonce=0 → tx nonce=1 is correct.
    match TransactionValidator::validate_against_state(
        &tx,
        Amount::from_tokens(100).unwrap(),
        Nonce::new(0),
        Timestamp::now(),
    ) {
        Ok(())  => println!("  [✓] State valid (balance=100, nonce=0→1)"),
        Err(e)  => println!("  [✗] {}", e),
    }

    // Failure case: insufficient balance.
    match TransactionValidator::validate_against_state(
        &tx,
        Amount::from_tokens(5).unwrap(), // only 5 tokens
        Nonce::new(0),
        Timestamp::now(),
    ) {
        Err(e)  => println!("  [✓] Correctly rejected: {}", e),
        Ok(())  => println!("  [✗] Should have failed"),
    }

    // Failure case: wrong nonce.
    match TransactionValidator::validate_against_state(
        &tx,
        Amount::from_tokens(100).unwrap(),
        Nonce::new(5), // account at nonce=5, tx has nonce=1
        Timestamp::now(),
    ) {
        Err(e)  => println!("  [✓] Correctly rejected: {}", e),
        Ok(())  => println!("  [✗] Should have failed"),
    }

    // ── 5. Mempool ────────────────────────────────────────────────────────────
    println!("\n── 5. Mempool ──────────────────────────────────────────");
    let mut pool = Mempool::with_defaults();

    // Add multiple transactions with different fees.
    let fees = [1_000u64, 5_000, 10_000, 2_000];
    let mut tx_ids = vec![];

    for fee_micro in fees {
        let kp   = KeyPair::generate().unwrap();
        let addr = crypto::Address::from_public_key(kp.public_key());
        // Use a different recipient to avoid self-transfer.
        let recip = recipient_addr.clone();
        let t = TransactionBuilder::new()
            .from_keypair(kp)
            .to(recip)
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(fee_micro).unwrap())
            .nonce(Nonce::new(0))
            .build()
            .unwrap();
        tx_ids.push(t.tx_id().clone());
        pool.add(t).unwrap();
    }

    println!("  Pool size : {}", pool.len());
    println!("  Top 4 by fee:");
    for t in pool.top_n(4) {
        println!("    fee={} | {}", t.fee(), hex::encode(&t.tx_id().as_bytes()[..6]));
    }

    // Remove confirmed transactions.
    pool.remove_batch(&tx_ids);
    println!("  After removing confirmed txs: pool size = {}", pool.len());
    println!();

    // ── 6. Receipt ────────────────────────────────────────────────────────────
    println!("── 6. Transaction Receipt ──────────────────────────────");
    let kp2 = KeyPair::generate().unwrap();
    let tx2 = TransactionBuilder::new()
        .from_keypair(kp2)
        .to(recipient_addr)
        .amount(Amount::from_tokens(1).unwrap())
        .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
        .nonce(Nonce::new(0))
        .build()
        .unwrap();

    let receipt = TransactionReceipt::success(
        tx2.tx_id().clone(),
        BlockHeight::new(42),
        crypto::sha256(b"block_42_hash"),
        tx2.fee(),
        Timestamp::now(),
    );
    println!("  {}", receipt);
    println!("  Success: {}", receipt.is_success());
    println!();

    println!("═══════════════════════════════════════════════════════");
    println!("  All checks passed.");
    println!("═══════════════════════════════════════════════════════");
}
