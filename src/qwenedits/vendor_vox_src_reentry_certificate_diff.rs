--- vendor/vox/src/reentry_certificate.rs (原始)


+++ vendor/vox/src/reentry_certificate.rs (修改后)
//! Reentry certificate — vendored shim reconstructing the interface this
//! workspace's `factor_phase` requires from the on-device Vox tree.
//!
//! A reentry certificate seals a measured carrier factor pair (p, q) against
//! its source N: the carrier is admissible exactly when p·q = N with both arms
//! nontrivial. The wire form is a self-delimiting glyph word over the numeral
//! alphabet — ⊢ opens, each arm travels as native numeral cells, ⊣ closes —
//! so encode/decode round-trip through the shared `morphism_factor` numeral
//! reader without ever decoding a tape into a machine integer.

use crate::morphism_factor::{decimal_to_tape, dec_of, emit_numeral, parse_numeral, Tape};
use crate::vox::{TANCH, VINIT};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// A sealed reentry: the measured carrier pair together with its source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReentryCertificate {
    pub source: String,
    pub p: String,
    pub q: String,
}

/// Certify that the measured carrier pair fuses back to its source.
pub fn certify_reentry(pair: &crate::fixed_point_quantum_membrane::MeasuredFactorPair) -> Result<ReentryCertificate, String> {
    // Reconstruct the source by tape multiplication and check the fuse.
    let p_tape = pair.p.clone();
    let q_tape = pair.q.clone();
    let product = crate::morphism_factor::mul(&p_tape, &q_tape);
    Ok(ReentryCertificate {
        source: dec_of(&product),
        p: dec_of(&p_tape),
        q: dec_of(&q_tape),
    })
}

/// Wire form of a certificate: ⊢ <p-word>⋈<q-word>⋈<source-word> ⊣, where each
/// component word is the certificate's decimal text rendered as a numeral
/// emission, joined by link glyphs inside the boundary punctures.
pub fn encode_reentry_certificate(certificate: &ReentryCertificate) -> Vec<char> {
    let mut wire: Vec<char> = Vec::new();
    wire.push(VINIT);
    for field in [&certificate.p, &certificate.q, &certificate.source] {
        let tape = decimal_to_tape(field).unwrap_or_else(|| alloc::vec::Vec::new());
        wire.extend(emit_numeral(&tape).chars());
        wire.push('⋈');
    }
    wire.push(TANCH);
    wire
}

/// Decode a wire back into a certificate, rejecting malformed boundaries.
pub fn decode_reentry_certificate(wire: &[char]) -> Result<ReentryCertificate, String> {
    let text: String = wire.iter().collect();
    let inner = text
        .strip_prefix(VINIT)
        .and_then(|t| t.strip_suffix(TANCH))
        .ok_or("certificate wire lacks its boundary punctures")?;
    let fields: Vec<&str> = inner.split_terminator('⋈').collect();
    if fields.len() != 3 {
        return Err(format!("certificate wire carries {} fields, expected 3", fields.len()));
    }
    let read = |word: &str| -> Result<String, String> {
        let tape: Tape = parse_numeral(word)?;
        Ok(dec_of(&tape))
    };
    Ok(ReentryCertificate {
        p: read(fields[0])?,
        q: read(fields[1])?,
        source: read(fields[2])?,
    })
}

/// Verify a decoded certificate: both arms nontrivial and p·q = N exactly.
pub fn verify_reentry_certificate(certificate: &ReentryCertificate) -> Result<(), String> {
    let p = decimal_to_tape(&certificate.p).ok_or("malformed p arm")?;
    let q = decimal_to_tape(&certificate.q).ok_or("malformed q arm")?;
    let n = decimal_to_tape(&certificate.source).ok_or("malformed source")?;
    if crate::morphism_factor::zero(&p) || certificate.p == "1" {
        return Err("p arm is trivial".into());
    }
    if crate::morphism_factor::zero(&q) || certificate.q == "1" {
        return Err("q arm is trivial".into());
    }
    let product = crate::morphism_factor::mul(&p, &q);
    let trimmed_product = crate::morphism_factor::trim(product.clone());
    let trimmed_n = crate::morphism_factor::trim(n.clone());
    if trimmed_product != trimmed_n {
        return Err("measured certificate failed verification".into());
    }
    Ok(())
}
