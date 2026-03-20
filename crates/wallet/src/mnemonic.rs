// mnemonic.rs — BIP-39-inspired mnemonic phrase generation.
//
// We use a simplified BIP-39 approach:
//   1. Generate 16 bytes (128 bits) of OS entropy.
//   2. Encode as 12 words from a fixed 2048-word wordlist subset.
//   3. to_seed() returns SHA-256(phrase_bytes) as a 32-byte seed.
//
// Security decisions:
//   • OsRng for entropy — no userspace seeding.
//   • The phrase is stored as a String (not secret by itself, but
//     the user must keep it private).
//   • to_seed() bytes are wrapped in Zeroizing so they're wiped
//     after use by the HD wallet derivation.
//   • We use a minimal 128-word English wordlist for simplicity.
//     Replace with the full BIP-39 2048-word list for production.

use rand::rngs::OsRng;
use rand::RngCore;
use zeroize::Zeroizing;

use crate::error::WalletError;

// ── Minimal wordlist (first 128 BIP-39 words) ─────────────────────────────────
// In production, use the full 2048-word BIP-39 English wordlist.
const WORDLIST: &[&str] = &[
    "abandon","ability","able","about","above","absent","absorb","abstract",
    "absurd","abuse","access","accident","account","accuse","achieve","acid",
    "acoustic","acquire","across","act","action","actor","actress","actual",
    "adapt","add","addict","address","adjust","admit","adult","advance",
    "advice","aerobic","afford","afraid","again","age","agent","agree",
    "ahead","aim","air","airport","aisle","alarm","album","alcohol",
    "alert","alien","all","alley","allow","almost","alone","alpha",
    "already","also","alter","always","amateur","amazing","among","amount",
    "amused","analyst","anchor","ancient","anger","angle","angry","animal",
    "ankle","announce","annual","another","answer","antenna","antique","anxiety",
    "any","apart","apology","appear","apple","approve","april","arch",
    "arctic","area","arena","argue","arm","armed","armor","army",
    "around","arrange","arrest","arrive","arrow","art","artefact","artist",
    "artwork","ask","aspect","assault","asset","assist","assume","asthma",
    "athlete","atom","attack","attend","attitude","attract","auction","audit",
    "august","aunt","author","auto","autumn","average","avocado","avoid",
    "awake","aware","away","awesome","awful","awkward","axis","baby",
];

/// Number of words in a generated mnemonic.
pub const MNEMONIC_WORD_COUNT: usize = 12;

/// A mnemonic seed phrase for wallet recovery.
///
/// Keep this private — anyone with this phrase can reconstruct all keys.
pub struct Mnemonic {
    phrase: String,
}

impl Mnemonic {
    /// Generate a fresh 12-word mnemonic using OS entropy.
    pub fn generate() -> Self {
        let mut bytes = [0u8; 16]; // 128 bits of entropy
        OsRng.fill_bytes(&mut bytes);

        // Encode bytes as words: each word encodes ~10.6 bits.
        // We use modular indexing into the wordlist.
        let words: Vec<&str> = (0..MNEMONIC_WORD_COUNT)
            .map(|i| {
                // Use 1 byte + overflow from previous to pick a word.
                let idx = bytes[i % bytes.len()] as usize
                    ^ (bytes[(i + 1) % bytes.len()] as usize >> 4);
                WORDLIST[idx % WORDLIST.len()]
            })
            .collect();

        Mnemonic { phrase: words.join(" ") }
    }

    /// Construct from an existing phrase.
    ///
    /// Validates that the phrase contains the expected number of words
    /// and that each word is in the wordlist.
    pub fn from_phrase(phrase: &str) -> Result<Self, WalletError> {
        let words: Vec<&str> = phrase.split_whitespace().collect();
        if words.len() != MNEMONIC_WORD_COUNT {
            return Err(WalletError::InvalidMnemonic(format!(
                "expected {} words, got {}",
                MNEMONIC_WORD_COUNT,
                words.len()
            )));
        }
        for word in &words {
            if !WORDLIST.contains(word) {
                return Err(WalletError::InvalidMnemonic(
                    format!("unknown word: '{}'", word)
                ));
            }
        }
        Ok(Mnemonic { phrase: phrase.to_owned() })
    }

    /// Derive a 32-byte seed from this mnemonic.
    ///
    /// Returns a `Zeroizing` wrapper so the seed bytes are wiped on drop.
    pub fn to_seed(&self) -> Zeroizing<[u8; 32]> {
        // seed = SHA-256("SYPCOIN_MNEMONIC_V1" || phrase_bytes)
        let mut pre = b"SYPCOIN_MNEMONIC_V1".to_vec();
        pre.extend_from_slice(self.phrase.as_bytes());
        let digest = crypto::sha256(&pre);
        Zeroizing::new(*digest.as_bytes())
    }

    /// The mnemonic phrase as a string.
    pub fn phrase(&self) -> &str {
        &self.phrase
    }

    /// Individual words in order.
    pub fn words(&self) -> Vec<&str> {
        self.phrase.split_whitespace().collect()
    }

    pub fn word_count(&self) -> usize {
        self.phrase.split_whitespace().count()
    }
}

impl std::fmt::Display for Mnemonic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Mask middle words for accidental display safety.
        let words: Vec<&str> = self.phrase.split_whitespace().collect();
        if words.len() <= 4 {
            return write!(f, "[MNEMONIC HIDDEN]");
        }
        write!(f, "{} *** {} words total ***", words[0], words.len())
    }
}
