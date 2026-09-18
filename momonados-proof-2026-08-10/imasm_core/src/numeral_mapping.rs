//! Native IMASM-Numeral Mapping — encode numerals to unique IMASM glyph words.
//!
//! Each numeral maps to a distinct glyph sequence following the Phase 4 algorithm:
//!   1. ⊢  VINIT        — initialize void_numeral
//!   2. ⊣  TANCH        — establish topological_boundary
//!   3. ≻×n AFWD        — increment_magnitude (n times)
//!   4. ⋈  CLINK        — digit_composition (multi-digit)
//!   5. ∈  FSPLIT       — parity_branch
//!   6. ⊤/⊥ EVALT/EVALF — evaluate even/odd parity
//!   7. ⊞  ENGAGR       — chiral_superposition
//!   8. ∋  FFUSE        — stoichiometric_rejoin
//!   9. ⊙  IMSCRIB      — critical_state (self-recognition)
//!  10. ≺×n AREV        — decrement_magnitude (back to void)
//!  11. ⊡  IFIX         — immutable_record
//!  12. ⋈  CLINK        — chain fixed record
//!  13. ⊣  TANCH        — re-anchor boundary

use crate::classic::Token;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// The canonical 14-glyph mapping program (the DECODER).
/// This is the fixed sequence from the blueprint that defines the numeral system.
pub const CANONICAL_MAPPING_WORD: &str = "⊢⊣≻⋈∈⊤⊥⊞∋⊙≺⊡⋈⊣";

/// Parity of a numeral.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Parity {
    Even,
    Odd,
}

/// A numeral with its IMASM encoding.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NumeralEncoding {
    pub value: i64,
    pub digits: Vec<u8>,
    pub magnitude: u64,
    pub parity: Parity,
    pub glyphs: Vec<Token>,
    pub glyph_word: String,
}

impl NumeralEncoding {
    /// Encode a numeral into its full IMASM glyph sequence.
    pub fn encode(value: i64) -> Self {
        let magnitude = value.unsigned_abs();
        let digits: Vec<u8> = magnitude.to_string().chars().filter_map(|c| c.to_digit(10)).map(|d| d as u8).collect();
        let parity = if value % 2 == 0 { Parity::Even } else { Parity::Odd };

        let mut glyphs = Vec::new();

        // Step 1-2: Boundary initialization
        glyphs.push(Token::Vinit);   // ⊢ void_numeral
        glyphs.push(Token::Tanch);   // ⊣ topological_boundary

        // Step 3: Build magnitude via AFWD (n times)
        for _ in 0..magnitude {
            glyphs.push(Token::Afwd);  // ≻ increment_magnitude
        }

        // Step 4: Digit composition (if multi-digit)
        if digits.len() > 1 {
            glyphs.push(Token::Clink);  // ⋈ digit_composition
            for _ in 1..digits.len() {
                glyphs.push(Token::Clink);
            }
        }

        // Step 5: Parity branch
        glyphs.push(Token::Fsplit);  // ∈ parity_branch

        // Step 6: Evaluate parity arm
        match parity {
            Parity::Even => glyphs.push(Token::Evalt),  // ⊤ even_parity
            Parity::Odd => glyphs.push(Token::Evalf),   // ⊥ odd_parity
        }

        // Step 7: Chiral superposition (always engaged)
        glyphs.push(Token::Engagr);  // ⊞ chiral_superposition

        // Step 8: Stoichiometric rejoin
        glyphs.push(Token::Ffuse);   // ∋ stoichiometric_rejoin

        // Step 9: Critical state - self-recognition
        glyphs.push(Token::Imscrib); // ⊙ critical_state

        // Step 10: Decrement magnitude back to void (for closure)
        for _ in 0..magnitude {
            glyphs.push(Token::Arev);  // ≺ decrement_magnitude
        }

        // Step 11: Immutable fixation
        glyphs.push(Token::Ifix);    // ⊡ immutable_record

        // Step 12: Chain fixed record
        glyphs.push(Token::Clink);   // ⋈ digit_composition (chaining)

        // Step 13: Re-anchor boundary
        glyphs.push(Token::Tanch);   // ⊣ topological_boundary

        let glyph_word = glyphs.iter().map(|t| t.code()).collect::<String>();

        Self {
            value,
            digits,
            magnitude,
            parity,
            glyphs,
            glyph_word,
        }
    }

    /// Encode with compact representation (minimal glyphs).
    pub fn encode_compact(value: i64) -> Self {
        let magnitude = value.unsigned_abs();
        let digits: Vec<u8> = magnitude.to_string().chars().filter_map(|c| c.to_digit(10)).map(|d| d as u8).collect();
        let parity = if value % 2 == 0 { Parity::Even } else { Parity::Odd };

        let mut glyphs = Vec::new();

        // ⊢ — init void
        glyphs.push(Token::Vinit);

        // ≻×n — magnitude
        for _ in 0..magnitude {
            glyphs.push(Token::Afwd);
        }

        // ⋈ — digit composition (if multi-digit)
        if digits.len() > 1 {
            glyphs.push(Token::Clink);
        }

        // ∈ — parity split
        glyphs.push(Token::Fsplit);

        // ⊤/⊥ — evaluate parity
        match parity {
            Parity::Even => glyphs.push(Token::Evalt),
            Parity::Odd => glyphs.push(Token::Evalf),
        }

        // ⊞ — chiral superposition
        glyphs.push(Token::Engagr);

        // ∋ — stoichiometric rejoin
        glyphs.push(Token::Ffuse);

        // ⊡ — immutable fixation
        glyphs.push(Token::Ifix);

        // ⊣ — re-anchor boundary
        glyphs.push(Token::Tanch);

        let glyph_word = glyphs.iter().map(|t| t.code()).collect::<String>();

        Self {
            value,
            digits,
            magnitude,
            parity,
            glyphs,
            glyph_word,
        }
    }

    /// Get the glyph word as a string.
    pub fn word(&self) -> &str {
        &self.glyph_word
    }

    /// Get the glyph count.
    pub fn len(&self) -> usize {
        self.glyphs.len()
    }
}

/// Machine for verifying numeral encodings by running the canonical program.
pub struct NumeralMachine {
    reg: RegClassic,
    fixed: bool,
    in_split: bool,
    split_arms: (RegClassic, RegClassic), // (even, odd)
    magnitude: u64,
    value: Option<i64>,
    parity: Option<Parity>,
    history: Vec<StepRecord>,
}

/// Classic 2-bit register (N, T, F, B)
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct RegClassic {
    t: bool,
    f: bool,
}

impl RegClassic {
    fn name(&self) -> &'static str {
        match (self.t, self.f) {
            (false, false) => "N",
            (true, false) => "T",
            (false, true) => "F",
            (true, true) => "B",
        }
    }

    #[allow(dead_code)]
    fn union(&mut self, other: RegClassic) {
        self.t |= other.t;
        self.f |= other.f;
    }

    fn clear(&mut self) {
        self.t = false;
        self.f = false;
    }
}

#[derive(Clone, Debug)]
pub struct StepRecord {
    pub glyph: &'static str,
    pub opcode: &'static str,
    pub action: String,
    pub reg_before: &'static str,
    pub reg_after: &'static str,
    pub magnitude: u64,
    pub value: Option<i64>,
    pub parity: Option<Parity>,
}

impl NumeralMachine {
    pub fn new() -> Self {
        Self {
            reg: RegClassic::default(),
            fixed: false,
            in_split: false,
            split_arms: (RegClassic::default(), RegClassic::default()),
            magnitude: 0,
            value: None,
            parity: None,
            history: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.reg = RegClassic::default();
        self.fixed = false;
        self.in_split = false;
        self.split_arms = (RegClassic::default(), RegClassic::default());
        self.magnitude = 0;
        self.value = None;
        self.parity = None;
        self.history.clear();
    }

    fn record(&mut self, glyph: &'static str, action: String, reg_before: RegClassic) {
        let opcode = match glyph {
            "⊢" => "VINIT", "⊣" => "TANCH", "≻" => "AFWD", "≺" => "AREV",
            "⋈" => "CLINK", "∈" => "FSPLIT", "⊤" => "EVALT", "⊥" => "EVALF",
            "⊞" => "ENGAGR", "∋" => "FFUSE", "⊙" => "IMSCRIB", "⊡" => "IFIX",
            _ => "?",
        };
        self.history.push(StepRecord {
            glyph,
            opcode,
            action,
            reg_before: reg_before.name(),
            reg_after: self.reg.name(),
            magnitude: self.magnitude,
            value: self.value,
            parity: self.parity,
        });
    }

    pub fn step(&mut self, token: Token) {
        if self.fixed && token != Token::Ifix && token != Token::Imscrib {
            return;
        }

        let reg_before = self.reg;
        let action = match token {
            Token::Vinit => {
                self.reg.clear();
                self.magnitude = 0;
                self.value = None;
                self.parity = None;
                self.in_split = false;
                self.split_arms = (RegClassic::default(), RegClassic::default());
                "Initialize void_numeral"
            }
            Token::Tanch => "Establish topological_boundary",
            Token::Afwd => {
                if self.magnitude == 0 {
                    self.magnitude = 1;
                    self.value = Some(1);
                } else {
                    self.magnitude += 1;
                    if let Some(v) = self.value { self.value = Some(v + 1); }
                }
                self.reg.t = true;
                &format!("Increment magnitude to {}", self.magnitude)
            }
            Token::Arev => {
                if self.magnitude > 0 {
                    self.magnitude = self.magnitude.saturating_sub(1);
                    if let Some(v) = self.value { self.value = Some(v.saturating_sub(1)); }
                }
                self.reg.clear();
                self.in_split = false;
                self.split_arms = (RegClassic::default(), RegClassic::default());
                &format!("Decrement magnitude to {}", self.magnitude)
            }
            Token::Clink => {
                if let Some(v) = self.value {
                    if v > 9 {
                        // Would split into digits
                    }
                }
                "Compose digits"
            }
            Token::Fsplit => {
                self.in_split = true;
                self.split_arms = (self.reg, self.reg);
                if let Some(v) = self.value {
                    if v % 2 == 0 {
                        self.split_arms.0.t = true; // even arm
                    } else {
                        self.split_arms.1.f = true; // odd arm
                    }
                }
                self.reg.t = true;
                self.reg.f = true;
                "Parity branch (even/odd)"
            }
            Token::Evalt => {
                if self.in_split {
                    self.split_arms.0.t = true;
                } else {
                    self.reg.t = true;
                }
                self.parity = Some(match self.parity {
                    Some(Parity::Odd) => Parity::Odd, // already odd
                    _ => Parity::Even,
                });
                "Evaluate even_parity"
            }
            Token::Evalf => {
                if self.in_split {
                    self.split_arms.1.f = true;
                } else {
                    self.reg.f = true;
                }
                self.parity = Some(match self.parity {
                    Some(Parity::Even) => Parity::Even,
                    _ => Parity::Odd,
                });
                "Evaluate odd_parity"
            }
            Token::Engagr => {
                if self.in_split {
                    self.split_arms.0.t = true;
                    self.split_arms.1.f = true;
                }
                self.reg.t = true;
                self.reg.f = true;
                self.parity = Some(Parity::Even); // Both
                "Engage chiral_superposition"
            }
            Token::Ffuse => {
                if self.in_split {
                    let (even, odd) = self.split_arms;
                    self.reg.t = even.t || odd.t;
                    self.reg.f = even.f || odd.f;
                    self.in_split = false;
                    self.split_arms = (RegClassic::default(), RegClassic::default());
                    self.parity = match (self.reg.t, self.reg.f) {
                        (true, true) => Some(Parity::Even), // both
                        (true, false) => Some(Parity::Even),
                        (false, true) => Some(Parity::Odd),
                        (false, false) => None,
                    };
                }
                "Stoichiometric rejoin"
            }
            Token::Imscrib => {
                if self.reg == RegClassic::default() {
                    self.reg.t = true;
                }
                "Critical state (self-recognition)"
            }
            Token::Ifix => {
                self.fixed = true;
                "Immutable fixation"
            }
            _ => "Unknown",
        };

        self.record(token.code(), action.to_string(), reg_before);
    }

    /// Run the canonical mapping program on this numeral.
    pub fn run_canonical(&mut self, numeral: i64) -> VerificationResult {
        // Pre-load the numeral state
        self.value = Some(numeral);
        self.magnitude = numeral.unsigned_abs();
        self.parity = if numeral % 2 == 0 { Some(Parity::Even) } else { Some(Parity::Odd) };

        // Parse canonical word
        let steps: Vec<Token> = CANONICAL_MAPPING_WORD
            .chars()
            .filter_map(|c| Token::parse(&c.to_string()))
            .collect();

        for tok in steps {
            self.step(tok);
        }

        let closed = self.reg == RegClassic::default();

        VerificationResult {
            input_numeral: numeral,
            final_register: self.reg.name().to_string(),
            final_magnitude: self.magnitude,
            final_value: self.value,
            final_parity: self.parity,
            closed,
            history: self.history.clone(),
            canonical_word: CANONICAL_MAPPING_WORD.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct VerificationResult {
    pub input_numeral: i64,
    pub final_register: String,
    pub final_magnitude: u64,
    pub final_value: Option<i64>,
    pub final_parity: Option<Parity>,
    pub closed: bool,
    pub history: Vec<StepRecord>,
    pub canonical_word: String,
}

/// Run the canonical mapping program on a numeral and return verification result.
pub fn run_canonical_mapping(numeral: i64) -> VerificationResult {
    let mut machine = NumeralMachine::new();
    machine.run_canonical(numeral)
}

/// Generate encodings for a range of numerals.
pub fn encode_range(start: i64, end: i64, compact: bool) -> Vec<NumeralEncoding> {
    (start..=end)
        .map(|v| {
            if compact {
                NumeralEncoding::encode_compact(v)
            } else {
                NumeralEncoding::encode(v)
            }
        })
        .collect()
}


/// Payload Extraction Operator Λ (Lambda)
/// ──────────────────────────────────────────────────────────────────────────────
///
/// Extracts the factor payload from a direct numeral word D(N):
///   D(N) = ⊢⊣ W_N ∈⊥⊞∋⊙≺⊡⋈⊣  (or ∈⊤⊞∋⊙≺⊡⋈⊣)
///   W_N  = body over {≻, ⋈} encoding factors via γ operator
///
/// γ: (p_bit, q_bit) → digram
///   γ(0,0) = ≻≻
///   γ(0,1) = ≻⋈
///   γ(1,0) = ⋈≻
///   γ(1,1) = ⋈⋈
///
/// Λ(W_N) deinterlaces W_N into two bitlanes (p, q) by splitting
/// the ≻/⋈ sequence into γ-digrams and mapping each digram back to (p_bit, q_bit).

const PREFIX: &str = "⊢⊣";
const SUFFIX_EVEN: &str = "∈⊤⊞∋⊙≺⊡⋈⊣";
const SUFFIX_ODD: &str = "∈⊥⊞∋⊙≺⊡⋈⊣";

/// γ operator: (p_bit, q_bit) → digram
fn gamma(p: u8, q: u8) -> &'static str {
    match (p, q) {
        (0, 0) => "≻≻",
        (0, 1) => "≻⋈",
        (1, 0) => "⋈≻",
        (1, 1) => "⋈⋈",
        _ => "≻≻",
    }
}

/// γ⁻¹: digram → (p_bit, q_bit)
fn gamma_inv(digram: &str) -> Option<(u8, u8)> {
    match digram {
        "≻≻" => Some((0, 0)),
        "≻⋈" => Some((0, 1)),
        "⋈≻" => Some((1, 0)),
        "⋈⋈" => Some((1, 1)),
        _ => None,
    }
}

/// Extract W_N from the full numeral word D(N).
fn extract_payload_body(word: &str) -> Result<&str, &'static str> {
    if !word.starts_with(PREFIX) {
        return Err("Word must start with ⊢⊣");
    }
    let body_start = PREFIX.len();
    
    let suffix = if word.ends_with(SUFFIX_EVEN) {
        SUFFIX_EVEN
    } else if word.ends_with(SUFFIX_ODD) {
        SUFFIX_ODD
    } else {
        return Err("Word must end with ∈⊤⊞∋⊙≺⊡⋈⊣ or ∈⊥⊞∋⊙≺⊡⋈⊣");
    };
    
    let body_end = word.len() - suffix.len();
    if body_end <= body_start {
        return Err("Empty body");
    }
    
    Ok(&word[body_start..body_end])
}

/// Λ(W_N) = extract factor payload via γ-deinterlacing.
/// Returns (bin_p, bin_q) as binary strings.
pub fn lambda_operator(body: &str) -> (String, String) {
    // Filter to only ≻ and ⋈
    let payload_glyphs: Vec<char> = body.chars().filter(|&c| c == '≻' || c == '⋈').collect();
    
    if payload_glyphs.is_empty() {
        return (String::new(), String::new());
    }
    
    let mut bin_p = String::new();
    let mut bin_q = String::new();
    
    // Split into digrams (pairs of glyphs)
    for i in (0..payload_glyphs.len()).step_by(2) {
        if i + 1 >= payload_glyphs.len() {
            break; // odd length, ignore trailing glyph
        }
        let digram: String = payload_glyphs[i..=i+1].iter().collect();
        if let Some((p_bit, q_bit)) = gamma_inv(&digram) {
            bin_p.push(char::from(b'0' + p_bit));
            bin_q.push(char::from(b'0' + q_bit));
        }
    }
    
    (bin_p, bin_q)
}

/// Reconstruct body from binary lanes using γ.
pub fn reconstruct_body(bin_p: &str, bin_q: &str) -> String {
    let max_len = bin_p.len().max(bin_q.len());
    let mut body = String::new();
    
    for i in 0..max_len {
        let p_bit = bin_p.chars().nth(i).unwrap_or('0') as u8 - b'0';
        let q_bit = bin_q.chars().nth(i).unwrap_or('0') as u8 - b'0';
        body.push_str(gamma(p_bit, q_bit));
    }
    body
}

/// Factor report from a direct numeral word (Grammar-native Λ operator).
/// Input: full D(N) word like "⊢⊣W∈⊥⊞∋⊙≺⊡⋈⊣"
/// Output: formatted report showing Λ(W_N) deinterlacing.
pub fn factor_report(word: &str) -> String {
    // Extract payload body W_N
    let body = match extract_payload_body(word) {
        Ok(b) => b,
        Err(e) => return format!("Error: {}", e),
    };
    
    // Apply Λ operator
    let (bin_p, bin_q) = lambda_operator(body);
    
    // Convert to integers
    let p = usize::from_str_radix(&bin_p, 2).unwrap_or(0);
    let q = usize::from_str_radix(&bin_q, 2).unwrap_or(0);
    let product = p * q;
    
    // Reconstruct body via γ
    let reconstructed = reconstruct_body(&bin_p, &bin_q);
    
    // Verify γ-reconstruction matches original body (filtered to ≻/⋈)
    let body_filtered: String = body.chars().filter(|&c| c == '≻' || c == '⋈').collect();
    let gamma_matches = reconstructed == body_filtered;
    
    // Build report
    let mut report = String::new();
    report.push_str(&format!("D(N) = {}\n", word));
    report.push_str(&format!("W_N  = {} (len={})\n", body, body.len()));
    report.push_str(&format!("Λ(W_N) = bin(p)={}, bin(q)={}\n", bin_p, bin_q));
    report.push_str(&format!("p = {}, q = {}\n", p, q));
    report.push_str(&format!("p × q = {}\n", product));
    report.push_str(&format!("γ-reconstructed = {}\n", reconstructed));
    report.push_str(&format!("γ matches body = {}\n", gamma_matches));
    
    report
}

/// Encode a numeral to its direct numeral word D(N) and return formatted report.
pub fn encode_report(n: i64) -> String {
    let enc = NumeralEncoding::encode(n);
    let mut report = String::new();
    report.push_str(&format!("N = {}\n", n));
    report.push_str(&format!("D(N) = {}\n", enc.glyph_word));
    report.push_str(&format!("glyph_count = {}\n", enc.glyphs.len()));
    report.push_str(&format!("parity = {:?}\n", enc.parity));
    report.push_str(&format!("magnitude = {}\n", enc.magnitude));
    report.push_str(&format!("digits = {:?}\n", enc.digits));
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_zero() {
        let enc = NumeralEncoding::encode(0);
        assert_eq!(enc.value, 0);
        assert_eq!(enc.parity, Parity::Even);
        assert!(enc.glyph_word.starts_with("⊢⊣"));
        assert!(enc.glyph_word.contains("∈"));
        assert!(enc.glyph_word.contains("⊤")); // even
        assert!(enc.glyph_word.contains("⊞"));
        assert!(enc.glyph_word.contains("∋"));
        assert!(enc.glyph_word.contains("⊙"));
        assert!(enc.glyph_word.contains("⊡"));
        assert!(enc.glyph_word.ends_with("⋈⊣"));
    }

    #[test]
    fn test_encode_one() {
        let enc = NumeralEncoding::encode(1);
        assert_eq!(enc.value, 1);
        assert_eq!(enc.parity, Parity::Odd);
        // Has one AFWD
        let afwd_count = enc.glyphs.iter().filter(|t| **t == Token::Afwd).count();
        assert_eq!(afwd_count, 1);
        // Has ⊥ (EVALF) for odd
        assert!(enc.glyphs.iter().any(|t| *t == Token::Evalf));
    }

    #[test]
    fn test_encode_parity_differs() {
        let even = NumeralEncoding::encode(2);
        let odd = NumeralEncoding::encode(3);
        assert_ne!(even.glyph_word, odd.glyph_word);
        // Even uses EVALT (⊤), odd uses EVALF (⊥)
        assert!(even.glyphs.iter().any(|t| *t == Token::Evalt));
        assert!(odd.glyphs.iter().any(|t| *t == Token::Evalf));
    }

    #[test]
    fn test_encode_magnitude_reflected() {
        let enc_5 = NumeralEncoding::encode(5);
        let enc_10 = NumeralEncoding::encode(10);
        // 10 has more AFWD than 5
        let afwd_5 = enc_5.glyphs.iter().filter(|t| **t == Token::Afwd).count();
        let afwd_10 = enc_10.glyphs.iter().filter(|t| **t == Token::Afwd).count();
        assert_eq!(afwd_5, 5);
        assert_eq!(afwd_10, 10);
    }

    #[test]
    fn test_encode_multi_digit_has_clink() {
        let enc_9 = NumeralEncoding::encode(9);
        let enc_10 = NumeralEncoding::encode(10);
        // Single digit: no CLINK or one
        let clink_9 = enc_9.glyphs.iter().filter(|t| **t == Token::Clink).count();
        // Multi-digit: has CLINK
        let clink_10 = enc_10.glyphs.iter().filter(|t| **t == Token::Clink).count();
        assert!(clink_10 > clink_9);
    }

    #[test]
    fn test_compact_encoding_shorter() {
        let full = NumeralEncoding::encode(42);
        let compact = NumeralEncoding::encode_compact(42);
        assert!(compact.glyphs.len() < full.glyphs.len());
    }

    #[test]
    fn test_canonical_mapping_closes() {
        let result = run_canonical_mapping(42);
        assert!(result.closed, "Canonical mapping should close for 42");
        assert_eq!(result.final_register, "N");
        assert_eq!(result.final_magnitude, 0);
    }

    #[test]
    fn test_canonical_mapping_various() {
        for n in [0, 1, 2, 7, 10, 42, 100, 256] {
            let result = run_canonical_mapping(n);
            assert!(result.closed, "Failed for {}", n);
            assert_eq!(result.final_register, "N");
        }
    }

    #[test]
    fn test_verify_returns_history() {
        let result = run_canonical_mapping(7);
        assert!(!result.history.is_empty());
        // Should have steps for each glyph in canonical word
        assert_eq!(result.history.len(), CANONICAL_MAPPING_WORD.chars().count());
    }
}