//! Native semiprime fixtures with independently verifiable prime certificates.
//! The factors stay in the control manifest; the membrane receives N alone.
use num_bigint::BigUint;
use num_traits::{One, Zero};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;

fn modular_power(mut base: u64, mut exponent: u64, modulus: u64) -> u64 {
    let mut result = 1u64;
    while exponent != 0 {
        if exponent & 1 != 0 {
            result = ((u128::from(result) * u128::from(base)) % u128::from(modulus)) as u64;
        }
        base = ((u128::from(base) * u128::from(base)) % u128::from(modulus)) as u64;
        exponent >>= 1;
    }
    result
}

// These seven witnesses give deterministic Miller-Rabin over the u64 domain.
// Candidates here always have their top bit set and are odd.
fn prime(candidate: u64) -> bool {
    let shifts = (candidate - 1).trailing_zeros();
    let odd = (candidate - 1) >> shifts;
    for witness in [2u64, 325, 9375, 28178, 450775, 9780504, 1795265022] {
        let base = witness % candidate;
        if base == 0 {
            continue;
        }
        let mut value = modular_power(base, odd, candidate);
        if value == 1 || value == candidate - 1 {
            continue;
        }
        let mut passed = false;
        for _ in 1..shifts {
            value = ((u128::from(value) * u128::from(value)) % u128::from(candidate)) as u64;
            if value == candidate - 1 {
                passed = true;
                break;
            }
        }
        if !passed {
            return false;
        }
    }
    true
}

fn draw_prime(entropy: &mut File, bits: usize) -> std::io::Result<u64> {
    loop {
        let mut bytes = [0u8; 8];
        entropy.read_exact(&mut bytes)?;
        let mask = if bits == 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        let candidate = (u64::from_le_bytes(bytes) & mask) | (1u64 << (bits - 1)) | 1;
        if prime(candidate) {
            return Ok(candidate);
        }
    }
}

struct PrimeCertificate {
    value: BigUint,
    // For a wide prime p, p-1 = 2*multiplier*q and q*q > p.
    proof: Option<(Box<PrimeCertificate>, BigUint, BigUint)>,
}

fn gcd(mut left: BigUint, mut right: BigUint) -> BigUint {
    while !right.is_zero() {
        let residue = left % &right;
        left = right;
        right = residue;
    }
    left
}

fn draw_below(entropy: &mut File, upper: &BigUint) -> std::io::Result<BigUint> {
    let bits = (upper - BigUint::one()).bits() as usize;
    if bits == 0 {
        return Ok(BigUint::zero());
    }
    let mask = (BigUint::one() << bits) - BigUint::one();
    let mut bytes = vec![0u8; bits.div_ceil(8)];
    loop {
        entropy.read_exact(&mut bytes)?;
        let value = BigUint::from_bytes_le(&bytes) & &mask;
        if &value < upper {
            return Ok(value);
        }
    }
}

fn certify_prime(entropy: &mut File, bits: usize) -> std::io::Result<PrimeCertificate> {
    if bits <= 64 {
        return Ok(PrimeCertificate {
            value: BigUint::from(draw_prime(entropy, bits)?),
            proof: None,
        });
    }
    // Pocklington's criterion: one certified divisor exceeding sqrt(p)
    // suffices. The random multiplier leaves the remaining part unrestricted.
    let q_bits = bits.checked_add(3).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "prime width exceeds host indexing"))? / 2;
    let q = certify_prime(entropy, q_bits)?;
    let two_q = &q.value << 1usize;
    let lower =
        ((BigUint::one() << (bits - 1)) - BigUint::one() + &two_q - BigUint::one()) / &two_q;
    let upper = ((BigUint::one() << bits) - BigUint::from(2u8)) / &two_q;
    let span = &upper - &lower + BigUint::one();
    loop {
        let multiplier = &lower + draw_below(entropy, &span)?;
        let value = &two_q * &multiplier + BigUint::one();
        let exponent = &value - BigUint::one();
        for witness in 2u64..=32 {
            let witness = BigUint::from(witness);
            if witness.modpow(&exponent, &value) != BigUint::one() {
                break;
            }
            let root = witness.modpow(&(&multiplier << 1usize), &value);
            if gcd(root - BigUint::one(), value.clone()) == BigUint::one() {
                return Ok(PrimeCertificate {
                    value,
                    proof: Some((Box::new(q), multiplier, witness)),
                });
            }
        }
    }
}

impl PrimeCertificate {
    fn from_json(record: &serde_json::Value) -> Result<Self, Box<dyn std::error::Error>> {
        let parse = |key: &str| -> Result<BigUint, Box<dyn std::error::Error>> {
            Ok(record
                .get(key)
                .and_then(serde_json::Value::as_str)
                .ok_or("certificate field missing")?
                .parse()?)
        };
        let value = parse("prime")?;
        let proof = match record.get("method").and_then(serde_json::Value::as_str) {
            Some("deterministic-u64-miller-rabin") => None,
            Some("pocklington") => Some((
                Box::new(Self::from_json(
                    record.get("q").ok_or("prime divisor certificate missing")?,
                )?),
                parse("multiplier")?,
                parse("witness")?,
            )),
            _ => return Err("unknown primality certificate method".into()),
        };
        Ok(Self { value, proof })
    }
    fn verify(&self) -> bool {
        match &self.proof {
            None => {
                self.value.bits() >= 2
                    && self.value.bits() <= 64
                    && self.value.bit(0)
                    && prime(self.value.to_u64_digits()[0])
            }
            Some((q, multiplier, witness)) => {
                if !q.verify()
                    || multiplier.is_zero()
                    || witness < &BigUint::from(2u8)
                    || witness >= &self.value
                    || &q.value * &q.value <= self.value
                    || (&q.value * multiplier << 1usize) + BigUint::one() != self.value
                {
                    return false;
                }
                let exponent = &self.value - BigUint::one();
                if witness.modpow(&exponent, &self.value) != BigUint::one() {
                    return false;
                }
                let root = witness.modpow(&(multiplier << 1usize), &self.value);
                !root.is_zero() && gcd(root - BigUint::one(), self.value.clone()) == BigUint::one()
            }
        }
    }

    fn json(&self) -> serde_json::Value {
        match &self.proof {
            None => {
                serde_json::json!({"prime": self.value.to_string(), "method": "deterministic-u64-miller-rabin"})
            }
            Some((q, multiplier, witness)) => serde_json::json!({"prime": self.value.to_string(),
                "method": "pocklington", "q": q.json(), "multiplier": multiplier.to_string(), "witness": witness.to_string()}),
        }
    }
}

fn verify_manifest(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let input = BufReader::new(File::open(path)?);
    let mut count = 0;
    for line in input.lines() {
        let record: serde_json::Value = serde_json::from_str(&line?)?;
        let p = PrimeCertificate::from_json(record.get("p").ok_or("p certificate missing")?)?;
        let q = PrimeCertificate::from_json(record.get("q").ok_or("q certificate missing")?)?;
        let n: BigUint = record
            .get("N")
            .and_then(serde_json::Value::as_str)
            .ok_or("semiprime source missing")?
            .parse()?;
        let bits = record
            .get("bits")
            .and_then(serde_json::Value::as_u64)
            .ok_or("semiprime width missing")?;
        if bits < 128
            || n.bits() != bits
            || p.value == q.value
            || &p.value * &q.value != n
            || !p.verify()
            || !q.verify()
        {
            return Err(format!("invalid semiprime certificate on record {count}").into());
        }
        count += 1;
    }
    if count == 0 {
        return Err("empty semiprime certificate manifest".into());
    }
    println!("Verified {count} persisted semiprime certificates; every input is at least 128 bits");
    Ok(())
}

fn generate() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let destination = args.next().ok_or(
        "usage: qpe_semiprime_cases <fresh_manifest.tsv> [count=16] [widths=128,192,256,512,1024]",
    )?;
    if destination == "--verify" {
        let path = args
            .next()
            .ok_or("usage: qpe_semiprime_cases --verify <manifest.proofs.jsonl>")?;
        if args.next().is_some() {
            return Err("unexpected certificate verifier argument".into());
        }
        return verify_manifest(&path);
    }
    let count = args
        .next()
        .map(|arg| arg.parse::<usize>())
        .transpose()?
        .unwrap_or(16);
    let widths = args
        .next()
        .unwrap_or_else(|| "128,192,256,512,1024".into())
        .split(',')
        .map(str::parse::<usize>)
        .collect::<Result<Vec<_>, _>>()?;
    if count == 0
        || widths.is_empty()
        || widths.iter().any(|bits| *bits < 128)
        || args.next().is_some()
    {
        return Err(
            "provide a positive case count and semiprime widths of at least 128 bits".into(),
        );
    }
    let proof_path = format!("{destination}.proofs.jsonl");
    let mut entropy = File::open("/dev/urandom")?;
    // Never replace an existing control set.
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(Path::new(&destination))?;
    let mut proofs = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(proof_path)?;
    writeln!(output, "case\tbits\tN\tp\tq")?;
    let mut seen = std::collections::BTreeSet::new();
    for index in 0..count {
        let bits = widths[index % widths.len()];
        let (n, p, q) = loop {
            let p = certify_prime(&mut entropy, bits / 2)?;
            let q = certify_prime(&mut entropy, bits - bits / 2)?;
            if !p.verify() || !q.verify() {
                return Err("prime certificate did not verify".into());
            }
            let n = &p.value * &q.value;
            if p.value != q.value && n.bits() == bits as u64 && seen.insert(n.clone()) {
                break (n, p, q);
            }
        };
        writeln!(
            proofs,
            "{}",
            serde_json::json!({"case": index, "bits": bits, "N": n.to_string(), "p": p.json(), "q": q.json()})
        )?;
        writeln!(output, "{index}\t{bits}\t{n}\t{}\t{}", p.value, q.value)?;
        println!("Certified case {index}: {bits}-bit semiprime");
    }
    output.sync_all()?;
    proofs.sync_all()?;
    println!("Generated {count} certified semiprimes in {destination}");
    Ok(())
}

fn main() {
    if let Err(error) = generate() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
