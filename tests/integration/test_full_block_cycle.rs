// integration/test_full_block_cycle.rs
// Full pipeline: genesis → transactions → mempool → block → execution → state

use block::BlockBuilder;
use consensus::{Blockchain, Miner};
use crypto::{Address, KeyPair};
use execution::{BlockExecutor, Executor};
use genesis::{GenesisLoader, GenesisState, GenesisBlock};
use primitives::{Amount, BlockHeight, Nonce, Timestamp};
use primitives::constants::MIN_TX_FEE_MICRO;
use state::WorldState;
use transaction::{Mempool, TransactionBuilder, TransactionValidator};

fn make_address() -> Address {
    Address::from_public_key(KeyPair::generate().unwrap().public_key())
}

/// Build a genesis state with one pre-funded account.
fn setup_genesis(funded_addr: &Address, tokens: u64) -> (WorldState, block::Block) {
    use genesis::GenesisAccount;
    let mut cfg = GenesisLoader::default_config();
    cfg.initial_accounts = vec![GenesisAccount {
        address: funded_addr.to_checksum_hex(),
        balance: format!("{}.000000", tokens),
        label:   None,
    }];
    let state      = GenesisState::build(&cfg).unwrap();
    let state_root = state.state_root().clone();
    let block      = GenesisBlock::build(&cfg, state_root).unwrap();
    (state, block)
}

#[test]
fn test_genesis_state_has_correct_balance() {
    let alice_kp   = KeyPair::generate().unwrap();
    let alice_addr = Address::from_public_key(alice_kp.public_key());

    let (state, _) = setup_genesis(&alice_addr, 1000);
    assert_eq!(state.get_balance(&alice_addr), Amount::from_tokens(1000).unwrap());
    assert!(state.verify_supply_invariant());
}

#[test]
fn test_transaction_through_mempool_to_block() {
    let sender_kp   = KeyPair::generate().unwrap();
    let sender_addr = Address::from_public_key(sender_kp.public_key());
    let recipient   = make_address();
    let miner_addr  = make_address();

    // ── Setup ─────────────────────────────────────────────────────────────────
    let (genesis_state, genesis_block) = setup_genesis(&sender_addr, 500);
    let mut executor  = Executor::new(genesis_state);
    let mut mempool   = Mempool::with_defaults();
    let mut blockchain = Blockchain::new(genesis_block.clone(), 1).unwrap();

    // ── Build 3 transactions ──────────────────────────────────────────────────
    for i in 1..=3u64 {
        let kp_i = KeyPair::generate().unwrap(); // different keypairs for simplicity
        let addr_i = Address::from_public_key(kp_i.public_key());
        let tx = TransactionBuilder::new()
            .from_keypair(kp_i)
            .to(recipient.clone())
            .amount(Amount::from_tokens(1).unwrap())
            .fee(Amount::from_micro(MIN_TX_FEE_MICRO).unwrap())
            .nonce(Nonce::new(i))
            .build()
            .unwrap();

        // Structural validation before mempool admission.
        TransactionValidator::validate_structure(&tx).unwrap();
        mempool.add(tx).unwrap();
    }
    assert_eq!(mempool.len(), 3);

    // ── Mine a block with those transactions ──────────────────────────────────
    executor.state(); // just to verify it's accessible
    let state_root = executor.state().state_root().clone();
    let txs: Vec<_> = mempool.top_n(10).iter().map(|t| (*t).clone()).collect();

    let template = BlockBuilder::new()
        .height(BlockHeight::new(1))
        .parent_hash(genesis_block.hash())
        .state_root(state_root)
        .miner(miner_addr.clone())
        .difficulty(1)
        .timestamp(Timestamp::from_millis(1_700_000_010_000))
        .transactions(txs.clone())
        .build()
        .unwrap();

    let miner  = Miner::new(miner_addr.clone());
    let result = miner.mine(template, || false).unwrap();
    let block  = result.block;

    // ── Add to blockchain ─────────────────────────────────────────────────────
    blockchain.add_block(block.clone()).unwrap();
    assert_eq!(blockchain.height(), BlockHeight::new(1));

    // ── Execute block ─────────────────────────────────────────────────────────
    executor.state(); // access before execution for comparison
    let receipt = executor.execute_block(&block).unwrap();

    // ── Verify ───────────────────────────────────────────────────────────────
    assert_eq!(executor.state().block_height(), BlockHeight::new(1));
    assert!(!receipt.reward_paid.is_zero(), "miner should receive block reward");
    assert!(executor.state().verify_supply_invariant());

    // Remove confirmed txs from mempool.
    let tx_ids: Vec<_> = txs.iter().map(|t| t.tx_id().clone()).collect();
    mempool.remove_batch(&tx_ids);
    assert!(mempool.is_empty());
}

#[test]
fn test_multi_block_chain() {
    let miner_addr = make_address();
    let mut cfg    = GenesisLoader::default_config();
    let state      = GenesisState::build(&cfg).unwrap();
    let root       = state.state_root().clone();
    let genesis    = GenesisBlock::build(&cfg, root).unwrap();

    let mut executor   = Executor::new(state);
    let mut blockchain = Blockchain::new(genesis.clone(), 1).unwrap();
    let miner          = Miner::new(miner_addr.clone());

    let mut prev_root = executor.state().state_root().clone();
    let mut parent    = genesis.hash();

    for h in 1..=5u64 {
        let state_root = executor.state().state_root().clone();
        let template = BlockBuilder::new()
            .height(BlockHeight::new(h))
            .parent_hash(parent)
            .state_root(state_root)
            .miner(miner_addr.clone())
            .difficulty(1)
            .timestamp(Timestamp::from_millis(1_700_000_000_000 + h * 10_000))
            .build()
            .unwrap();

        let block = miner.mine(template, || false).unwrap().block;
        parent    = block.hash();

        blockchain.add_block(block.clone()).unwrap();
        executor.execute_block(&block).unwrap();

        // State root must change with each block (reward applied).
        let new_root = executor.state().state_root().clone();
        assert_ne!(new_root.as_bytes(), prev_root.as_bytes(),
            "state_root must change after block {}", h);
        prev_root = new_root;

        assert_eq!(blockchain.height().as_u64(), h);
        assert!(executor.state().verify_supply_invariant());
    }

    // After 5 blocks, miner should have earned rewards.
    assert!(!executor.state().get_balance(&miner_addr).is_zero());
}

#[test]
fn test_block_reward_accumulates() {
    use primitives::block_reward_at;

    let miner_addr = make_address();
    let mut cfg    = GenesisLoader::default_config();
    let state      = GenesisState::build(&cfg).unwrap();
    let root       = state.state_root().clone();
    let genesis    = GenesisBlock::build(&cfg, root).unwrap();

    let mut executor = Executor::new(state);
    let miner        = Miner::new(miner_addr.clone());
    let mut parent   = genesis.hash();

    for h in 1..=3u64 {
        let template = BlockBuilder::new()
            .height(BlockHeight::new(h))
            .parent_hash(parent)
            .state_root(executor.state().state_root().clone())
            .miner(miner_addr.clone())
            .difficulty(1)
            .timestamp(Timestamp::from_millis(1_700_000_000_000 + h * 10_000))
            .build()
            .unwrap();

        let block = miner.mine(template, || false).unwrap().block;
        parent    = block.hash();
        executor.execute_block(&block).unwrap();
    }

    // Expected: 3 × block_reward_at(height=1..3).
    let expected = (1..=3u64)
        .map(|h| block_reward_at(&BlockHeight::new(h)).as_micro())
        .sum::<u64>();

    assert_eq!(executor.state().get_balance(&miner_addr).as_micro(), expected);
}
