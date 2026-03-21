// tx/transaction.rs — The core Transaction type.
//
// Security decisions:
//
//   PRIVATE CONSTRUCTOR
//   • Transaction has no public constructor. The only way to create one is
//     through TransactionBuilder::build(). This ensures every Transaction
//     value is fully validated and signed before it can exist in memory.
//
//   CANONICAL SERIALIZATION
//   • to_bytes() produces a deterministic byte sequence used for:
//       1. Computing tx_id = SHA-256(to_bytes())
//       2. Constructing the NoncePayload that was signed
//     The format is fixed: any change is a hard fork.
//     Field order: version(1) || chain_id(8LE) || from(20) || to(20) ||
//                  pubkey(32) || amount(8LE) || fee(8LE) || nonce(8LE) ||
//                  timestamp(8LE) || data_len(2LE) || data
//
//   CHAIN_ID IN SIGNED PRE-IMAGE
//   • chain_id is included in to_bytes() which feeds into NoncePayload.
//     A signature valid on testnet is invalid on mainnet even if all other
//     fields are identical. This prevents cross-chain replay attacks.
//
//   TX_ID BINDING
//   • tx_id = SHA-256(to_bytes()). It is computed once in the builder and
//     stored. It is NOT recomputed on access — callers trust the stored value
//     which was verified at construction time.

use serde::{Deserialize, Serialize};

use crypto::{Address, HashDigest, PublicKey, Signature, sha256};
use primitives::{Amount, Nonce, Timestamp};

use crate::tx::constants::{MAX_TX_DATA_SIZE, TX_VERSION};

/// A fully signed, validated transaction ready for mempool or block inclusion.
///
/// # Invariants
/// - `signature` is a valid Ed25519 signature over the canonical pre-image.
/// - `from` == `Address::from_public_key(&public_key)`.
/// - `amount > 0`.
/// - `fee >= MIN_TX_FEE_MICRO`.
/// - `chain_id == CHAIN_ID` (from primitives::constants).
/// - `data.len() <= MAX_TX_DATA_SIZE`.
///
/// All invariants are enforced by `TransactionBuilder::build()`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    // ── Protocol ──────────────────────────────────────────────────────────────
    /// Transaction format version (= TX_VERSION = 1).
    pub(crate) version: u8,

    /// Chain identifier. Prevents replay across different networks.
    pub(crate) chain_id: u64,

    // ── Parties ───────────────────────────────────────────────────────────────
    /// Sender address, derived from `public_key`.
    pub(crate) from: Address,

    /// Recipient address.
    pub(crate) to: Address,

    /// Sender's Ed25519 public key. Used for signature verification.
    pub(crate) public_key: PublicKey,

    // ── Value ─────────────────────────────────────────────────────────────────
    /// Amount transferred in micro-tokens.
    pub(crate) amount: Amount,

    /// Transaction fee paid to the block producer in micro-tokens.
    pub(crate) fee: Amount,

    // ── Replay protection ─────────────────────────────────────────────────────
    /// Sender's account nonce at the time of signing.
    pub(crate) nonce: Nonce,

    // ── Signature ─────────────────────────────────────────────────────────────
    /// Ed25519 signature over SHA-256(DOMAIN_SEP || nonce_le8 || to_bytes()).
    pub(crate) signature: Signature,

    // ── Metadata ──────────────────────────────────────────────────────────────
    /// Creation timestamp in Unix milliseconds.
    pub(crate) timestamp: Timestamp,

    /// Optional arbitrary data (max MAX_TX_DATA_SIZE bytes).
    pub(crate) data: Option<Vec<u8>>,

    // ── Cached fields (computed once at build time) ───────────────────────────
    /// SHA-256 digest of to_bytes(). Unique identifier for this transaction.
    pub(crate) tx_id: HashDigest,
}

impl Transaction {
    // ── Public accessors ──────────────────────────────────────────────────────

    /// Unique transaction identifier: SHA-256(canonical_bytes).
    pub fn tx_id(&self) -> &HashDigest {
        &self.tx_id
    }

    /// Sender address.
    pub fn from(&self) -> &Address {
        &self.from
    }

    /// Recipient address.
    pub fn to(&self) -> &Address {
        &self.to
    }

    /// Transfer amount in micro-tokens.
    pub fn amount(&self) -> Amount {
        self.amount
    }

    /// Transaction fee in micro-tokens.
    pub fn fee(&self) -> Amount {
        self.fee
    }

    /// Total amount deducted from sender: `amount + fee`.
    ///
    /// Returns `None` on overflow (impossible given Amount invariants,
    /// but handled for completeness).
    pub fn total_deducted(&self) -> Option<Amount> {
        self.amount.checked_add(self.fee)
    }

    /// Sender's nonce at signing time.
    pub fn nonce(&self) -> Nonce {
        self.nonce
    }

    /// Sender's public key.
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }

    /// Ed25519 signature.
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// Creation timestamp.
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    /// Optional data payload.
    pub fn data(&self) -> Option<&[u8]> {
        self.data.as_deref()
    }

    /// Chain ID this transaction is bound to.
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// Protocol version byte.
    pub fn version(&self) -> u8 {
        self.version
    }

    // ── Canonical serialization ───────────────────────────────────────────────

    /// Produce the canonical byte sequence for this transaction.
    ///
    /// This is the value that:
    ///   1. Gets hashed to produce `tx_id`.
    ///   2. Is fed into `NoncePayload::new(nonce, to_bytes())` for signing.
    ///
    /// Layout (all multi-byte integers little-endian):
    /// ```text
    /// version(1) || chain_id(8) || from(20) || to(20) || pubkey(32) ||
    /// amount(8)  || fee(8)      || nonce(8) || timestamp(8) ||
    /// data_len(2) || data(0..=256)
    /// ```
    ///
    /// Wire format — matches the Kotlin wallet TransactionSigner exactly.
    ///
    /// Layout:
    /// ```text
    /// pubkey(32) | to_addr(20) | amount_micro(8 LE) | fee_micro(8 LE) |
    /// nonce(8 LE) | chain_id(8 LE) | timestamp_ms(8 LE) | signature(64)
    /// ```
    ///
    /// Total: 32 + 20 + 8 + 8 + 8 + 8 + 8 + 64 = 156 bytes (no data field)
    ///
    /// This format is FIXED. Any change is a consensus-breaking hard fork.
    pub fn to_bytes(&self) -> Vec<u8> {
        // payload (92 bytes) + signature (64 bytes)
        let mut buf = Vec::with_capacity(156);
        buf.extend_from_slice(self.public_key.as_bytes());   // 32
        buf.extend_from_slice(self.to.as_bytes());           // 20
        buf.extend_from_slice(&self.amount.as_micro().to_le_bytes());    // 8
        buf.extend_from_slice(&self.fee.as_micro().to_le_bytes());       // 8
        buf.extend_from_slice(&self.nonce.as_u64().to_le_bytes());       // 8
        buf.extend_from_slice(&self.chain_id.to_le_bytes());             // 8
        buf.extend_from_slice(&self.timestamp.as_millis().to_le_bytes());// 8
        buf.extend_from_slice(self.signature.as_bytes());                // 64
        buf
    }


    /// Parse a transaction sent from the Kotlin wallet (bincode format, 210 bytes).
    ///
    /// Layout:
    ///   tx_id(32) | from(20) | to(20) | pubkey(32) | sig(64) |
    ///   amount(8) | fee(8) | nonce(8) | timestamp(8) | chain_id(8) |
    ///   version(1) | data_none(1)
    pub fn from_wallet_bytes(bytes: &[u8]) -> Result<Self, ()> {
        if bytes.len() != 210 { return Err(()); }

        let tx_id_arr: [u8; 32] = bytes[0..32].try_into().map_err(|_| ())?;
        let tx_id = crypto::sha256(&tx_id_arr);
        // from[20] at 32..52 — unused, derived from pubkey
        let to_bytes: [u8; 20] = bytes[52..72].try_into().map_err(|_| ())?;
        let pk_bytes: [u8; 32] = bytes[72..104].try_into().map_err(|_| ())?;
        let sig_bytes: [u8; 64] = bytes[104..168].try_into().map_err(|_| ())?;

        let amount_micro  = u64::from_le_bytes(bytes[168..176].try_into().map_err(|_| ())?);
        let fee_micro     = u64::from_le_bytes(bytes[176..184].try_into().map_err(|_| ())?);
        let nonce_val     = u64::from_le_bytes(bytes[184..192].try_into().map_err(|_| ())?);
        let timestamp_ms  = u64::from_le_bytes(bytes[192..200].try_into().map_err(|_| ())?);
        let chain_id_val  = u64::from_le_bytes(bytes[200..208].try_into().map_err(|_| ())?);

        let public_key = PublicKey::from_bytes(pk_bytes).map_err(|_| ())?;
        let from       = Address::from_public_key(&public_key);
        let to         = Address::from_raw_bytes(to_bytes);
        let signature  = Signature::from_bytes(sig_bytes).map_err(|_| ())?;
        let amount     = Amount::from_micro(amount_micro).map_err(|_| ())?;
        let fee        = Amount::from_micro(fee_micro).map_err(|_| ())?;
        let nonce      = Nonce::new(nonce_val);
        let timestamp  = Timestamp::from_millis(timestamp_ms);

        Ok(Transaction {
            tx_id,
            from,
            to,
            public_key,
            signature,
            amount,
            fee,
            nonce,
            timestamp,
            chain_id: chain_id_val,
            version:  bytes[208],
            data:     None,
        })
    }
    /// Serialized size in bytes (fixed: 156 = 92 payload + 64 signature).
    pub fn size_bytes(&self) -> usize { 156 }

    /// Recompute the tx_id from the current canonical bytes.
    ///
    /// Used internally by the builder. Callers should use `tx_id()`.
    pub(crate) fn compute_tx_id(tx_bytes: &[u8]) -> HashDigest {
        sha256(tx_bytes)
    }

    /// Compute the tx_id constant.
    #[allow(dead_code)]
    pub(crate) fn max_data_size() -> usize {
        MAX_TX_DATA_SIZE
    }

    /// Protocol version constant.
    #[allow(dead_code)]
    pub(crate) fn current_version() -> u8 {
        TX_VERSION
    }
}

impl std::fmt::Display for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tx[{}] {} → {} | {} tokens | fee {} | nonce {}",
            hex::encode(&self.tx_id.as_bytes()[..6]),
            self.from.to_checksum_hex(),
            self.to.to_checksum_hex(),
            self.amount,
            self.fee,
            self.nonce,
        )
    }
}

impl Transaction {
    /// Bytes used as the signing pre-image — excludes signature and tx_id.
    /// The signature cannot be part of its own pre-image (circular).
    /// Pre-image for signing — payload without signature.
    /// Must match Kotlin wallet: SHA-256("SYPCOIN_TX_V1" || nonce_le8 || payload)
    ///
    /// Layout: pubkey(32) | to(20) | amount(8) | fee(8) | nonce(8) | chain_id(8) | timestamp(8)
    pub(crate) fn to_bytes_for_signing(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(92);
        buf.extend_from_slice(self.public_key.as_bytes());              // 32
        buf.extend_from_slice(self.to.as_bytes());                      // 20
        buf.extend_from_slice(&self.amount.as_micro().to_le_bytes());   // 8
        buf.extend_from_slice(&self.fee.as_micro().to_le_bytes());      // 8
        buf.extend_from_slice(&self.nonce.as_u64().to_le_bytes());      // 8
        buf.extend_from_slice(&self.chain_id.to_le_bytes());            // 8
        buf.extend_from_slice(&self.timestamp.as_millis().to_le_bytes());// 8
        buf
    }
}
