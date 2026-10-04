use g_momonados::anyon_pair::{FibonacciPair, PairMatrix, COMPUTATIONAL_CHANNELS};
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Signed, ToPrimitive, Zero};
#[allow(dead_code)]
pub mod work;

pub struct Metrics { pub computational: f64, pub leakage: f64, pub closure: f64, pub exchanges: usize }

pub fn numeral(word: &str) -> Result<BigUint, String> {
    let decoded = g_momonados::godel_calculus::decode(word).map_err(|e| e.to_string())?;
    if !matches!(decoded.structure, g_momonados::godel_calculus::Structure::CellBinary { .. }) {
        return Err("prepared value must be a cell-binary word".into());
    }
    if g_momonados::godel_calculus::encode_cell_binary(&decoded.value) != word {
        return Err("prepared value must be a canonical cell-binary word".into());
    }
    Ok(decoded.value.bits_le().iter().enumerate().fold(BigUint::zero(), |n, (bit, set)| {
        if *set { n | (BigUint::one() << bit) } else { n }
    }))
}
pub fn signed_numeral(value: &str) -> Result<BigInt, String> {
    let mut chars = value.chars();
    let sign = match chars.next() {
        Some('≺') => num_bigint::Sign::Minus,
        Some('≻') => num_bigint::Sign::Plus,
        _ => return Err("signed coordinate must begin with an IMASM direction".into()),
    };
    Ok(BigInt::from_biguint(sign, numeral(chars.as_str())?))
}
fn ratio(n: &BigInt, d: &BigInt) -> f64 {
    let shift = n.bits().max(d.bits()).saturating_sub(900) as usize;
    (n >> shift).to_f64().unwrap_or(0.0) / (d >> shift).to_f64().unwrap_or(1.0)
}

pub fn contract(prepared: &serde_json::Value) -> Result<(BigUint, PairMatrix, Metrics), String> {
    let raw = prepared["source_word"].as_str().ok_or("missing source word")?;
    let n = numeral(raw)?;
    if let Some(operator) = prepared.get("prepared_operator") {
        let format = g_momonados::phase_unbraid::FixedPointFormat::for_modulus(&n)?;
        if numeral(operator["w_bits_word"].as_str().ok_or("missing precision word")?)?.to_u64() != Some(format.w_bits) {
            return Err("prepared operator precision differs from its source".into());
        }
        let entries = operator["matrix"].as_array().ok_or("missing prepared operator entries")?;
        let mut values = Vec::new();
        for entry in entries {
            let coordinate = |field: &str| -> Result<BigInt, String> {
                signed_numeral(entry[field].as_str().ok_or_else(|| format!("missing operator {field}"))?)
            };
            values.push(g_momonados::phase_unbraid::FixedComplex {
                re: coordinate("re_word")?, im: coordinate("im_word")?,
            });
        }
        let matrix = PairMatrix(values.try_into().map_err(|_| "prepared operator must contain all five fusion channels")?);
        let integer = |field: &str| -> Result<BigUint, String> {
            numeral(operator[field].as_str().ok_or_else(|| format!("missing prepared {field}"))?)
        };
        let metric = |field: &str| -> Result<f64, String> {
            let value = f64::from_bits(integer(field)?.to_u64().ok_or("invalid diagnostic word")?);
            if value.is_finite() && value >= 0.0 { Ok(value) }
            else { Err(format!("invalid prepared {field}")) }
        };
        let metrics = Metrics { computational: metric("computational_word")?, leakage: metric("leakage_word")?,
            closure: metric("closure_word")?, exchanges: integer("exchanges_word")?.to_usize()
                .ok_or("invalid prepared exchange count")? };
        return Ok((n, matrix, metrics));
    }
    let word: Vec<i32> = if let Some(exchanges) = prepared["exchange_words"].as_array() {
        exchanges.iter().map(|value| {
            let generator = signed_numeral(value.as_str().ok_or("exchange must be an IMASM word")?)?
                .to_i32().ok_or("invalid exchange word")?;
            if !(1..=5).contains(&generator.unsigned_abs()) {
                return Err("exchange word is outside the six-strand braid".into());
            }
            Ok(generator)
        }).collect::<Result<_, String>>()?
    } else {
        return Err("Fourier preparation requires IMASM exchange words".into());
    };
    if word.is_empty() { return Err("Fourier preparation has no exchange words".into()); }
    let accuracy = numeral(prepared["accuracy_word"].as_str().ok_or("missing accuracy word")?)?
        .to_u64().ok_or("invalid accuracy word")?;
    let exponent = i32::try_from(accuracy).map_err(|_| "accuracy overflow")?;
    let pair = FibonacciPair::new(&n)?;
    let physical = pair.evaluate(&word)?;
    let observed = pair.in_pair_channels(&physical);
    let scale = pair.format().scale();
    let mut overlap = (0.0f64, 0.0f64);
    for (k, &row) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
        for (l, &col) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            let angle = core::f64::consts::FRAC_PI_2 * (k * l) as f64;
            let z = &observed.0[5 * row + col];
            let (re, im) = (ratio(&z.re, &scale), ratio(&z.im, &scale));
            overlap.0 += (re * angle.cos() + im * angle.sin()) * 0.5;
            overlap.1 += (im * angle.cos() - re * angle.sin()) * 0.5;
        }
    }
    let norm = overlap.0.hypot(overlap.1);
    if !norm.is_finite() || norm == 0.0 { return Err("zero Fourier overlap".into()); }
    let phase = (overlap.0 / norm, overlap.1 / norm);
    let mut computational = 0.0f64;
    for (k, &row) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
        for (l, &col) in COMPUTATIONAL_CHANNELS.iter().enumerate() {
            let angle = core::f64::consts::FRAC_PI_2 * (k * l) as f64;
            let ideal = ((phase.0 * angle.cos() - phase.1 * angle.sin()) * 0.5,
                         (phase.0 * angle.sin() + phase.1 * angle.cos()) * 0.5);
            let z = &observed.0[5 * row + col];
            computational = computational.max((ratio(&z.re, &scale) - ideal.0)
                .hypot(ratio(&z.im, &scale) - ideal.1));
        }
    }
    let leakage = ratio(&BigInt::from(observed.leakage()), &scale);
    let inverse: Vec<_> = word.iter().rev().map(|g| -*g).collect();
    let returned = pair.evaluate(&inverse)?.multiply(&physical, pair.format());
    let identity = PairMatrix::identity(pair.format());
    let closure = returned.0.iter().zip(identity.0.iter()).map(|(a,b)| {
        ratio(&(&a.re - &b.re).abs().max((&a.im - &b.im).abs()), &scale)
    }).fold(0.0f64, f64::max);
    if !computational.is_finite() || !leakage.is_finite() || !closure.is_finite()
        || computational.max(leakage).max(closure) > 2.0f64.powi(-exponent) {
        return Err(format!("Fourier membrane residual: computational={computational:.8e} leakage={leakage:.8e} return={closure:.8e}"));
    }
    Ok((n, observed, Metrics { computational, leakage, closure, exchanges: word.len() }))
}
