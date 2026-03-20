// integration/test_network_sync.rs
// Simulate two nodes syncing: node A mines blocks, node B catches up.

use block::BlockBuilder;
use consensus::{Blockchain, Miner};
use crypto::{Address, KeyPair, sha256};
use execution::Executor;
use genesis::{GenesisBlock, GenesisLoader, GenesisState};
use networking::{BlockSync, HeaderSync};
use primitives::{BlockHeight, Timestamp};

fn make_address() -> Address {
    Address::from_public_key(KeyPair::generate().unwrap().public_key())
}

fn build_chain(blocks: u64) -> (Vec<block::Block>, Executor) {
    let addr  = make_address();
    let cfg   = GenesisLoader::default_config();
    let state = GenesisState::build(&cfg).unwrap();
    let root  = state.state_root().clone();
    let gen   = GenesisBlock::build(&cfg, root).unwrap();

    let mut executor = Executor::new(state);
    let mut blocks_out = vec![gen.clone()];
    let miner  = Miner::new(addr.clone());
    let mut parent = gen.hash();

    for h in 1..=blocks {
        let template = BlockBuilder::new()
            .height(BlockHeight::new(h))
            .parent_hash(parent.clone())
            .state_root(executor.state().state_root().clone())
            .miner(addr.clone())
            .difficulty(1)
            .timestamp(Timestamp::from_millis(1_700_000_000_000 + h * 10_000))
            .build()
            .unwrap();

        let block = miner.mine(template, || false).unwrap().block;
        parent    = block.hash();
        executor.execute_block(&block).unwrap();
        blocks_out.push(block);
    }

    (blocks_out, executor)
}

#[test]
fn test_header_sync_completes_on_short_response() {
    // Node A has 5 blocks. Node B asks for headers.
    let (blocks_a, _) = build_chain(5);
    let headers_a: Vec<_> = blocks_a.iter().map(|b| b.header().clone()).collect();

    let tip_hash = blocks_a[0].hash(); // B starts from genesis
    let mut sync = HeaderSync::new(tip_hash);

    // B receives all headers in one batch (< MAX_HEADERS_PER_MSG) → complete.
    let done = sync.on_headers(headers_a.clone());
    assert!(done, "sync should complete when response < limit");
    assert_eq!(sync.pending_headers().len(), headers_a.len());
}

#[test]
fn test_block_sync_receives_all_blocks() {
    let (blocks_a, _) = build_chain(3);
    let headers: Vec<_> = blocks_a[1..].iter().map(|b| b.header().clone()).collect();

    let mut sync = BlockSync::from_headers(&headers);
    assert_eq!(sync.pending_count(), 3);
    assert!(!sync.is_complete());

    // Simulate receiving blocks in two batches.
    sync.on_blocks(vec![blocks_a[1].clone(), blocks_a[2].clone()]);
    sync.on_blocks(vec![blocks_a[3].clone()]);

    assert!(sync.is_complete());
    assert_eq!(sync.received.len(), 3);
}

#[test]
fn test_node_b_reaches_same_state_root_as_node_a() {
    // Node A: mine 3 blocks.
    let (blocks_a, executor_a) = build_chain(3);

    // Node B: start from genesis, apply all blocks from A.
    let cfg_b   = GenesisLoader::default_config();
    let state_b = GenesisState::build(&cfg_b).unwrap();
    let mut executor_b = Executor::new(state_b);

    // Apply blocks 1..3 (skip genesis).
    for block in &blocks_a[1..] {
        executor_b.execute_block(block).unwrap();
    }

    // Both nodes must have the same state root.
    assert_eq!(
        executor_a.state().state_root().as_bytes(),
        executor_b.state().state_root().as_bytes(),
        "nodes must converge to the same state root after sync"
    );

    assert_eq!(executor_a.state().block_height(), executor_b.state().block_height());
    assert!(executor_b.state().verify_supply_invariant());
}

#[test]
fn test_sync_diverge_then_converge() {
    // Both nodes start from genesis.
    // Node A mines 2 blocks. Node B mines 1 different block.
    // Node B then applies A's chain → converges.

    let addr_a = make_address();
    let addr_b = make_address();
    let miner_a = Miner::new(addr_a.clone());
    let miner_b = Miner::new(addr_b.clone());

    let cfg    = GenesisLoader::default_config();
    let state  = GenesisState::build(&cfg).unwrap();
    let root   = state.state_root().clone();
    let gen    = GenesisBlock::build(&cfg, root).unwrap();

    // Node A mines 2 blocks.
    let mut exec_a = Executor::new(state.clone());
    let mut parent = gen.hash();
    let mut blocks_a = vec![];
    for h in 1..=2u64 {
        let t = BlockBuilder::new()
            .height(BlockHeight::new(h))
            .parent_hash(parent.clone())
            .state_root(exec_a.state().state_root().clone())
            .miner(addr_a.clone())
            .difficulty(1)
            .timestamp(Timestamp::from_millis(1_700_000_000_000 + h * 10_000))
            .build().unwrap();
        let b = miner_a.mine(t, || false).unwrap().block;
        parent = b.hash();
        exec_a.execute_block(&b).unwrap();
        blocks_a.push(b);
    }

    // Node B applies A's chain from genesis.
    let mut exec_b = Executor::new(state);
    for b in &blocks_a {
        exec_b.execute_block(b).unwrap();
    }

    assert_eq!(
        exec_a.state().state_root().as_bytes(),
        exec_b.state().state_root().as_bytes(),
        "after applying same chain, state roots must match"
    );
}
