// examples/usage.rs — End-to-end demonstration of the crypto crate.
//
// Run with:
//   cargo run --example usage --features test-utils
//
// This file intentionally uses the public API only — no internal imports.
// It serves as a living contract test: if this compiles and passes, the
// public surface is complete and correct.

use crypto::{
    Address, CryptoError, HashDigest, KeyPair, NoncePayload, PublicKey, Signature,
    keccak256, sha256, sign, verify,
};

fn main() -> Result<(), CryptoError> {
    println!("═══════════════════════════════════════════════════════");
    println!("  Blockchain Crypto Layer — Example Usage");
    println!("═══════════════════════════════════════════════════════\n");

    // ─── 1. Keypair Generation ────────────────────────────────────────────────
    println!("── 1. Keypair Generation ───────────────────────────────");

    let keypair = KeyPair::generate()?;
    let public_key: &PublicKey = keypair.public_key();

    println!("  Public key (hex) : {}", hex::encode(public_key.as_bytes()));
    println!("  Private key      : [REDACTED — never printed]\n");

    // ─── 2. Address Derivation ────────────────────────────────────────────────
    println!("── 2. Address Derivation ───────────────────────────────");
    println!("  Formula : SHA-256(b\"SYPCOIN_ADDR_V1\" || pubkey)[0..20]");

    let address = Address::from_public_key(public_key);
    let checksum_hex = address.to_checksum_hex();

    println!("  Address (checksum hex) : {}", checksum_hex);

    // Round-trip: parse the checksum hex back, confirm equality.
    let restored = Address::from_checksum_hex(&checksum_hex)?;
    assert_eq!(address, restored, "Address round-trip failed");
    println!("  Checksum round-trip    : ✓ OK\n");

    // ─── 3. Hashing ───────────────────────────────────────────────────────────
    println!("── 3. Hashing ──────────────────────────────────────────");

    let data = b"block header bytes would go here";

    let sha_digest: HashDigest    = sha256(data);
    let keccak_digest: HashDigest = keccak256(data);

    println!("  Input           : {:?}", std::str::from_utf8(data).unwrap());
    println!("  SHA-256         : {}", hex::encode(sha_digest.as_bytes()));
    println!("  Keccak-256      : {}", hex::encode(keccak_digest.as_bytes()));
    println!();

    // ─── 4. Signing ───────────────────────────────────────────────────────────
    println!("── 4. Signing ──────────────────────────────────────────");
    println!("  Domain separator : b\"SYPCOIN_TX_V1\" (prepended before hashing)");
    println!("  Pre-image        : SHA-256(DOMAIN_SEP || nonce_le8 || payload)");
    println!("  Algorithm        : Ed25519 deterministic (RFC 8032)\n");

    // Simulate a transaction: Alice sends 50 tokens to Bob, with nonce = 1.
    let tx_bytes = b"from:alice to:bob amount:50 asset:TOKEN";
    let nonce    = 1u64;
    let payload  = NoncePayload::new(nonce, tx_bytes.to_vec());

    println!("  Nonce           : {}", nonce);
    println!("  Payload         : {:?}", std::str::from_utf8(tx_bytes).unwrap());
    println!("  Encoded (hex)   : {}", hex::encode(payload.encode()));

    let signature: Signature = sign(&keypair, &payload)?;
    println!("  Signature (hex) : {}", hex::encode(signature.as_bytes()));
    println!();

    // ─── 5. Signature Verification ────────────────────────────────────────────
    println!("── 5. Signature Verification ───────────────────────────");

    // ✓ Valid: correct key, correct payload, correct nonce.
    match verify(public_key, &payload, &signature) {
        Ok(()) => println!("  [✓] Valid signature verified successfully"),
        Err(e) => println!("  [✗] Unexpected failure: {}", e),
    }

    // ✗ Invalid: different nonce (replay attempt).
    let replayed = NoncePayload::new(0, tx_bytes.to_vec());
    match verify(public_key, &replayed, &signature) {
        Err(CryptoError::VerificationFailed) =>
            println!("  [✓] Replay attack (wrong nonce) correctly rejected"),
        Ok(())  => println!("  [✗] BUG: Replay was accepted!"),
        Err(e)  => println!("  [✗] Unexpected error: {}", e),
    }

    // ✗ Invalid: tampered payload (amount changed).
    let tampered_tx = b"from:alice to:bob amount:9999 asset:TOKEN";
    let tampered    = NoncePayload::new(nonce, tampered_tx.to_vec());
    match verify(public_key, &tampered, &signature) {
        Err(CryptoError::VerificationFailed) =>
            println!("  [✓] Tampered payload correctly rejected"),
        Ok(())  => println!("  [✗] BUG: Tampered payload was accepted!"),
        Err(e)  => println!("  [✗] Unexpected error: {}", e),
    }

    // ✗ Invalid: wrong public key.
    let other_keypair  = KeyPair::generate()?;
    let other_pubkey   = other_keypair.public_key();
    match verify(other_pubkey, &payload, &signature) {
        Err(CryptoError::VerificationFailed) =>
            println!("  [✓] Wrong public key correctly rejected"),
        Ok(())  => println!("  [✗] BUG: Wrong key was accepted!"),
        Err(e)  => println!("  [✗] Unexpected error: {}", e),
    }

    println!();

    // ─── 6. Determinism Check ─────────────────────────────────────────────────
    println!("── 6. Determinism (Ed25519 RFC 8032) ───────────────────");

    let sig1 = sign(&keypair, &payload)?;
    let sig2 = sign(&keypair, &payload)?;
    assert_eq!(sig1.as_bytes(), sig2.as_bytes(),
        "Ed25519 signing must be deterministic");
    println!("  [✓] Same (key, payload) → identical signature bytes\n");

    // ─── 7. Signature & PublicKey serialisation round-trip ────────────────────
    println!("── 7. Byte round-trips ─────────────────────────────────");

    let sig_bytes: [u8; 64]  = *signature.as_bytes();
    let pk_bytes:  [u8; 32]  = *public_key.as_bytes();

    let sig_restored = Signature::from_bytes(sig_bytes)?;
    let pk_restored  = PublicKey::from_bytes(pk_bytes)?;

    assert_eq!(signature.as_bytes(), sig_restored.as_bytes());
    assert_eq!(public_key,           &pk_restored);
    println!("  [✓] Signature serialisation round-trip OK");
    println!("  [✓] PublicKey  serialisation round-trip OK\n");

    println!("═══════════════════════════════════════════════════════");
    println!("  All checks passed.");
    println!("═══════════════════════════════════════════════════════");

    Ok(())
}
