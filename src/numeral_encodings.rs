//! Shared numeral forms used by tfactor and SRFP/DNN production.
use alloc::string::String;
use num_bigint::BigUint;
use num_traits::Zero;

/// Each hex nibble's canonical IMASM word, one word per bit pattern of
/// {T-bit, F-bit, I-bit, fork-openness-bit}, built from the ob3ect
/// "A hex digit's fourth bit as fork openness, not a fifth glyph"
/// (`ob3ect/digital/a_hex_digit_s_fourth_bit_as_fork_openness_not_a_f03510ea`).
/// v=0xF reproduces that ob3ect's own canonical word exactly — checked, not
/// assumed. The template: ⊢, then ∈ iff the fork bit is set, then ⊤ iff the
/// T-bit is set, then ≻⋈, then ⊥ iff the F-bit is set, then ≺ (the clear —
/// banked if the fork opened before it, exposed otherwise), then ⋈ and ∋
/// iff the fork bit is set, then ⊞ iff the I-bit is set, then ⊙⊡⊣.
/// Verified pairwise distinct across all 16 against the real kernel.
pub const HEX_WORDS: [&str; 16] = [
    "⊢≻⋈≺⊙⊡⊣",             // 0x0  T=0 F=0 I=0 fork=0
    "⊢⊤≻⋈≺⊙⊡⊣",           // 0x1  T=1 F=0 I=0 fork=0
    "⊢≻⋈⊥≺⊙⊡⊣",           // 0x2  T=0 F=1 I=0 fork=0
    "⊢⊤≻⋈⊥≺⊙⊡⊣",         // 0x3  T=1 F=1 I=0 fork=0
    "⊢≻⋈≺⊞⊙⊡⊣",           // 0x4  T=0 F=0 I=1 fork=0
    "⊢⊤≻⋈≺⊞⊙⊡⊣",         // 0x5  T=1 F=0 I=1 fork=0
    "⊢≻⋈⊥≺⊞⊙⊡⊣",         // 0x6  T=0 F=1 I=1 fork=0
    "⊢⊤≻⋈⊥≺⊞⊙⊡⊣",       // 0x7  T=1 F=1 I=1 fork=0
    "⊢∈≻⋈≺⋈∋⊙⊡⊣",         // 0x8  T=0 F=0 I=0 fork=1
    "⊢∈⊤≻⋈≺⋈∋⊙⊡⊣",       // 0x9  T=1 F=0 I=0 fork=1
    "⊢∈≻⋈⊥≺⋈∋⊙⊡⊣",       // 0xA  T=0 F=1 I=0 fork=1
    "⊢∈⊤≻⋈⊥≺⋈∋⊙⊡⊣",     // 0xB  T=1 F=1 I=0 fork=1
    "⊢∈≻⋈≺⋈⊞∋⊙⊡⊣",       // 0xC  T=0 F=0 I=1 fork=1
    "⊢∈⊤≻⋈≺⋈⊞∋⊙⊡⊣",     // 0xD  T=1 F=0 I=1 fork=1
    "⊢∈≻⋈⊥≺⋈⊞∋⊙⊡⊣",     // 0xE  T=0 F=1 I=1 fork=1
    "⊢∈⊤≻⋈⊥≺⋈⊞∋⊙⊡⊣",   // 0xF  T=1 F=1 I=1 fork=1 -- the ob3ect's own word
];

/// Convert N (decimal string) to hex, then concatenate each nibble's
/// canonical word, most-significant nibble first.
pub fn digit_encode(n_str: &str) -> String {
    let n: BigUint = match n_str.parse() { Ok(v) => v, Err(_) => return String::new() };
    let hex = n.to_str_radix(16);
    let mut out = String::new();
    for c in hex.chars() {
        if let Some(d) = c.to_digit(16) {
            out.push_str(HEX_WORDS[d as usize]);
        }
    }
    out
}

/// Invert the existing fixed motifs; canonicality rejects added high zero digits.
pub fn digit_decode(word:&str)->Option<BigUint> {
    if word.is_empty(){return None;}
    let mut n=BigUint::zero();
    for motif in word.split_inclusive('⊣') {
        let digit=HEX_WORDS.iter().position(|candidate|*candidate==motif)?;
        n=(n<<4usize)+BigUint::from(digit);
    }
    if digit_encode(&n.to_string())!=word{return None;}
    Some(n)
}

#[derive(Clone,Debug,PartialEq,Eq,PartialOrd,Ord)]
pub struct SourceForms {pub native:String,pub hex:String}
impl SourceForms {
    pub fn from_n(n:&BigUint)->Result<Self,String> {
        let decimal=n.to_string();
        let forms=Self{native:crate::native_numeral::encode(&decimal),hex:digit_encode(&decimal)};
        if crate::native_numeral::decode(&forms.native).as_ref()!=Some(n)
            ||digit_decode(&forms.hex).as_ref()!=Some(n) {
            return Err("tfactor numeral forms failed exact source return".into());
        }
        Ok(forms)
    }
    pub fn source_value(&self)->Result<BigUint,String> {
        let native=crate::native_numeral::decode(&self.native).ok_or("native source word no longer decodes")?;
        let hex=digit_decode(&self.hex).ok_or("hex source word no longer decodes")?;
        if native!=hex{return Err("the two source forms disagree".into());}
        Ok(native)
    }
}
