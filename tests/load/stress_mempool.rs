// load/stress_mempool.rs
// Stress test: fill mempool with 1000 transactions and verify ordering.

use crypto::{Address, KeyPair};
use primitives::{Amount, Nonce, Timestamp};
use primitives::constants::MIN_TX_FEE_MICRO;
use transaction::{Mempool, MempoolConfig, TransactionBuilder};

fn make_address() -> Address {
    Address::from_public_key(KeyPair::generate().unwrap().public_key())
}

#[test]
fn test_mempool_1000_transactions() {
    let config = MempoolConfig {
        max_size:        2000,
        max_per_address: 64,
        min_fee:         Amount::from_micro(MIN_TX_FEE_MICRO).unwrap(),
        tx_ttl_ms:       300_000,
    };
    let mut pool = Mempool::new(config);

    let start = Timestamp::now();

    // Add 1000 transactions from different senders.
    for i in 0..1000u64 {
        let kp  = KeyPair::generate().unwrap();
        let fee = MIN_TX_FEE_MICRO + (i % 100) * 100; // varying fees
        let tx  = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_address())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(fee).unwrap())
            .nonce(Nonce::new(1))
            .build()
            .unwrap();
        pool.add(tx).unwrap();
    }

    let elapsed = Timestamp::now().as_millis().saturating_sub(start.as_millis());
    println!("[stress_mempool] 1000 txs added in {}ms", elapsed);
    assert_eq!(pool.len(), 1000);

    // Verify top-N ordering: highest fee first.
    let top10 = pool.top_n(10);
    assert_eq!(top10.len(), 10);
    for window in top10.windows(2) {
        assert!(
            window[0].fee().as_micro() >= window[1].fee().as_micro(),
            "top_n must return highest-fee transactions first"
        );
    }
}

#[test]
fn test_mempool_max_capacity_respected() {
    let config = MempoolConfig {
        max_size:        100,
        max_per_address: 64,
        min_fee:         Amount::from_micro(MIN_TX_FEE_MICRO).unwrap(),
        tx_ttl_ms:       300_000,
    };
    let mut pool = Mempool::new(config);

    for i in 0..100u64 {
        let kp = KeyPair::generate().unwrap();
        let tx = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_address())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(i + 1))
            .build()
            .unwrap();
        pool.add(tx).unwrap();
    }

    assert_eq!(pool.len(), 100);
    assert!(pool.is_full());

    // 101st should be rejected.
    let kp_overflow = KeyPair::generate().unwrap();
    let overflow_tx = TransactionBuilder::new()
        .from_keypair(kp_overflow)
        .to(make_address())
        .amount(Amount::from_tokens(1).unwrap())
        .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
        .nonce(Nonce::new(1))
        .build()
        .unwrap();
    assert!(pool.add(overflow_tx).is_err());
}

#[test]
fn test_mempool_eviction_removes_expired() {
    let config = MempoolConfig {
        max_size:        1000,
        max_per_address: 64,
        min_fee:         Amount::from_micro(MIN_TX_FEE_MICRO).unwrap(),
        tx_ttl_ms:       1, // 1ms TTL — everything expires immediately
    };
    let mut pool = Mempool::new(config);

    for _ in 0..10u64 {
        let kp = KeyPair::generate().unwrap();
        let tx = TransactionBuilder::new()
            .from_keypair(kp)
            .to(make_address())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(1))
            .build()
            .unwrap();
        pool.add(tx).unwrap();
    }

    std::thread::sleep(std::time::Duration::from_millis(5));
    let evicted = pool.evict_expired(Timestamp::now());
    assert_eq!(evicted, 10, "all expired transactions must be evicted");
    assert!(pool.is_empty());
}
