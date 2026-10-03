//! Exact sieve witnesses and arithmetic between numeral frames.
use super::{encode_cell_binary, Nat};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cmp::Ordering;

impl Nat {
    fn bit(&self, position: usize) -> bool {
        self.bits_le().get(position).copied().unwrap_or(false)
    }

    fn mod_small(&self, modulus: usize) -> usize {
        let mut residue = 0usize;
        for &bit in self.bits_le().iter().rev() {
            residue = if residue >= modulus - residue {
                residue - (modulus - residue)
            } else {
                residue + residue
            };
            if bit {
                residue = if residue == modulus - 1 {
                    0
                } else {
                    residue + 1
                };
            }
        }
        residue
    }

    fn mod_nat(&self, modulus: &Self) -> Self {
        self.div_rem(modulus).expect("nonzero sieve divisor").1
    }

    /// Exact binary division; zero divisors have no quotient.
    pub fn div_rem(&self, divisor: &Self) -> Option<(Self, Self)> {
        if divisor.is_zero() {
            return None;
        }
        let mut quotient = alloc::vec![false; self.bits_le().len()];
        let mut remainder = Self::zero();
        for position in (0..self.bits_le().len()).rev() {
            remainder = remainder.shl(1);
            if self.bit(position) {
                remainder = remainder.add(&Self::one());
            }
            if remainder.cmp_nat(divisor) != Ordering::Less {
                remainder = remainder
                    .sub(divisor)
                    .expect("ordered remainder subtraction");
                quotient[position] = true;
            }
        }
        Some((Self::from_bits_le(quotient), remainder))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimeSieveRead {
    pub aperture_width: usize,
    pub aperture: Nat,
    pub tested_primes: Nat,
    pub divisor: Option<Nat>,
    pub lower_bound: Option<Nat>,
}

fn nat_from_usize(mut value: usize) -> Nat {
    let mut bits = Vec::new();
    while value != 0 {
        bits.push(value & 1 == 1);
        value >>= 1;
    }
    Nat::from_bits_le(bits)
}

fn power_of_two(exponent: usize) -> Result<Nat, String> {
    let bit_count = exponent
        .checked_add(1)
        .ok_or_else(|| "sieve aperture exceeds addressable memory".to_string())?;
    let mut bits = Vec::new();
    bits.try_reserve_exact(bit_count)
        .map_err(|_| "insufficient memory for sieve aperture".to_string())?;
    bits.resize(exponent, false);
    bits.push(true);
    Ok(Nat::from_bits_le(bits))
}

fn integer_sqrt(value: usize) -> usize {
    let mut low = 0usize;
    let mut high = value;
    while low < high {
        let distance = high - low;
        let middle = low + distance / 2 + distance % 2;
        if middle <= value / middle {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    low
}

fn segmented_odd_sieve(value: &Nat, aperture: usize) -> Result<(Nat, Option<Nat>), String> {
    const SEGMENT_ODDS: usize = 32 * 1024;
    let root = integer_sqrt(aperture);
    let base_len = root
        .checked_add(1)
        .ok_or_else(|| "sieve base range exceeds addressable memory".to_string())?;
    let mut composite = Vec::new();
    composite
        .try_reserve_exact(base_len)
        .map_err(|_| "insufficient memory for sieve base primes".to_string())?;
    composite.resize(base_len, false);
    let mut primes = Vec::new();
    for candidate in 2..=root {
        if composite[candidate] {
            continue;
        }
        if primes.len() == primes.capacity() {
            primes
                .try_reserve(1)
                .map_err(|_| "insufficient memory for sieve base-prime list".to_string())?;
        }
        primes.push(candidate);
        if candidate <= root / candidate {
            let mut multiple = candidate * candidate;
            while multiple <= root {
                composite[multiple] = true;
                multiple += candidate;
            }
        }
    }

    let mut tested = Nat::zero();
    let mut low = 3usize;
    let mut segment = Vec::new();
    segment
        .try_reserve_exact(SEGMENT_ODDS)
        .map_err(|_| "insufficient memory for sieve segment".to_string())?;
    while low <= aperture {
        let span = 2usize.saturating_mul(SEGMENT_ODDS - 1);
        let mut high = low.saturating_add(span).min(aperture);
        if high & 1 == 0 {
            high -= 1;
        }
        let count = (high - low) / 2 + 1;
        segment.clear();
        segment.resize(count, false);
        for &prime in &primes {
            if prime > high / prime {
                break;
            }
            if prime == 2 {
                continue;
            }
            let square = prime * prime;
            let quotient = low / prime;
            let first_multiple = if low % prime == 0 {
                low
            } else {
                quotient
                    .checked_add(1)
                    .and_then(|next| next.checked_mul(prime))
                    .unwrap_or(usize::MAX)
            };
            let first = square.max(first_multiple);
            let first = if first & 1 == 0 {
                first.checked_add(prime).unwrap_or(usize::MAX)
            } else {
                first
            };
            let step = prime * 2;
            let mut multiple = first;
            while multiple <= high {
                segment[(multiple - low) / 2] = true;
                let Some(next) = multiple.checked_add(step) else {
                    break;
                };
                multiple = next;
            }
        }
        for (offset, marked) in segment.iter().copied().enumerate() {
            if !marked {
                let candidate = low + offset * 2;
                tested = tested.add(&Nat::one());
                if value.mod_small(candidate) == 0 {
                    return Ok((tested, Some(nat_from_usize(candidate))));
                }
            }
        }
        if high >= aperture - 1 {
            break;
        }
        low = high.saturating_add(2);
    }
    Ok((tested, None))
}

fn arbitrary_nat_sieve(value: &Nat, aperture: &Nat) -> Result<(Nat, Option<Nat>), String> {
    let two = Nat::from_u64(2);
    let mut candidate = Nat::from_u64(3);
    let mut tested_primes = Nat::zero();
    let mut primes_and_squares: Vec<(Nat, Nat)> = Vec::new();
    while candidate.cmp_nat(aperture) != Ordering::Greater {
        let mut is_prime = true;
        for (prime, square) in &primes_and_squares {
            if square.cmp_nat(&candidate) == Ordering::Greater {
                break;
            }
            if candidate.mod_nat(prime).is_zero() {
                is_prime = false;
                break;
            }
        }
        if is_prime {
            tested_primes = tested_primes.add(&Nat::one());
            if value.mod_nat(&candidate).is_zero() {
                return Ok((tested_primes, Some(candidate)));
            }
            if primes_and_squares.len() == primes_and_squares.capacity() {
                primes_and_squares
                    .try_reserve(1)
                    .map_err(|_| "insufficient memory for arbitrary sieve primes".to_string())?;
            }
            primes_and_squares.push((candidate.clone(), candidate.mul(&candidate)));
        }
        candidate = candidate.add(&two);
    }
    Ok((tested_primes, None))
}

/// Read the odd-prime sieve lane directly from the canonical bit support.
/// A returned lower bound is backed by testing every odd prime through 2^width.
pub fn prime_sieve_read(value: &Nat, width: usize) -> Result<PrimeSieveRead, String> {
    if width < 2 {
        return Err("sieve width must be at least 2".to_string());
    }
    let aperture = power_of_two(width)?;
    let fast_aperture = (width < usize::BITS as usize).then(|| 1usize << width);
    let (tested_primes, divisor) = if let Some(fast_aperture) = fast_aperture {
        let (tested, divisor) = segmented_odd_sieve(value, fast_aperture)?;
        (tested, divisor)
    } else {
        arbitrary_nat_sieve(value, &aperture)?
    };
    let odd_part = Nat::from_bits_le(
        value
            .bits_le()
            .iter()
            .skip(value.bits_le().iter().take_while(|bit| !**bit).count())
            .copied()
            .collect(),
    );
    let lower_bound = if divisor.is_none() && odd_part.cmp_nat(&Nat::one()) == Ordering::Greater {
        Some(aperture.clone())
    } else {
        None
    };
    Ok(PrimeSieveRead {
        aperture_width: width,
        aperture,
        tested_primes,
        divisor,
        lower_bound,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactorPairRead {
    pub factor: Nat,
    pub cofactor: Nat,
    pub product_closed: bool,
    pub semiprime_closed: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameArithmeticRead {
    pub source: Nat,
    pub left_width: usize,
    pub left_index: usize,
    pub left: Nat,
    pub operator: &'static str,
    pub right_width: usize,
    pub right_index: usize,
    pub right: Nat,
    pub result: Nat,
    pub remainder: Option<Nat>,
    pub return_frames: Vec<FrameReturnRead>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameReturnRead {
    pub width: usize,
    pub groups: usize,
    pub recovered: Nat,
    pub closed: bool,
}

fn frame_bits(value: &Nat) -> Vec<bool> {
    if value.is_zero() {
        alloc::vec![false]
    } else {
        value.bits_le().to_vec()
    }
}

fn frame_group(value: &Nat, width: usize, index: usize) -> Result<Nat, String> {
    let bits = frame_bits(value);
    let start = index
        .checked_mul(width)
        .ok_or_else(|| "frame group index exceeds addressable memory".to_string())?;
    if start >= bits.len() {
        return Err(format!(
            "frame group {index} is outside width-{width} frame"
        ));
    }
    let end = start.saturating_add(width).min(bits.len());
    Ok(Nat::from_bits_le(bits[start..end].to_vec()))
}

fn frame_return(value: &Nat, width: usize) -> Result<FrameReturnRead, String> {
    let bits = frame_bits(value);
    let groups = bits.len().div_ceil(width);
    let digits = (0..groups)
        .map(|index| frame_group(value, width, index))
        .collect::<Result<Vec<_>, _>>()?;
    let recovered = Nat::from_bits_le(
        (0..bits.len())
            .map(|position| digits[position / width].bit(position % width))
            .collect(),
    );
    Ok(FrameReturnRead {
        width,
        groups,
        closed: recovered == *value,
        recovered,
    })
}

pub fn frame_arithmetic_read(
    value: &Nat,
    left_width: usize,
    left_index: usize,
    operator: &str,
    right_width: usize,
    right_index: usize,
) -> Result<FrameArithmeticRead, String> {
    if left_width < 2 || right_width < 2 {
        return Err("frame widths must be at least 2".to_string());
    }
    if left_width == right_width {
        return Err("frame arithmetic requires values from different widths".to_string());
    }
    let left = frame_group(value, left_width, left_index)?;
    let right = frame_group(value, right_width, right_index)?;
    let (operator, result, remainder) = match operator {
        "add" => ("add", left.add(&right), None),
        "mul" => ("mul", left.mul(&right), None),
        "sub" => (
            "sub",
            left.sub(&right)
                .ok_or_else(|| "frame subtraction underflow".to_string())?,
            None,
        ),
        "mod" => {
            let (_, remainder) = left
                .div_rem(&right)
                .ok_or_else(|| "frame modulo by zero".to_string())?;
            ("mod", remainder.clone(), Some(remainder))
        }
        "divmod" => {
            let (quotient, remainder) = left
                .div_rem(&right)
                .ok_or_else(|| "frame divmod by zero".to_string())?;
            ("divmod", quotient, Some(remainder))
        }
        _ => return Err("frame operator must be add|mul|sub|mod|divmod".to_string()),
    };
    let return_frames = (2..=8)
        .map(|width| frame_return(&result, width))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(FrameArithmeticRead {
        source: value.clone(),
        left_width,
        left_index,
        left,
        operator,
        right_width,
        right_index,
        right,
        result,
        remainder,
        return_frames,
    })
}

pub fn render_frame_arithmetic(read: &FrameArithmeticRead) -> String {
    let remainder = read
        .remainder
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| "none".to_string());
    let mut output = format!(
        "frame-left   width={} group={} value={} word={}\n\
         frame-right  width={} group={} value={} word={}\n\
         operation    {}\n\
         result       {}\n\
         result-word  {}\n\
         remainder    {}\n",
        read.left_width,
        read.left_index,
        read.left,
        encode_cell_binary(&read.left),
        read.right_width,
        read.right_index,
        read.right,
        encode_cell_binary(&read.right),
        read.operator,
        read.result,
        encode_cell_binary(&read.result),
        remainder,
    );
    for returned in &read.return_frames {
        output.push_str(&format!(
            "frame-return width={} groups={} recovered={} closure={} word={}\n",
            returned.width,
            returned.groups,
            returned.recovered,
            if returned.closed { "closed" } else { "open" },
            encode_cell_binary(&returned.recovered),
        ));
    }
    output
}

pub fn frame_arithmetic_from_args(args: &[&str]) -> Result<FrameArithmeticRead, String> {
    if args.len() != 7 || args.first().copied() != Some("frame-op") {
        return Err(
            "godel frame-op <natural-number> <left-width> <left-group> <add|mul|sub|mod|divmod> <right-width> <right-group>".to_string(),
        );
    }
    let value =
        Nat::from_decimal(args[1]).ok_or_else(|| format!("not a natural number: {}", args[1]))?;
    let parse_address = |raw: &str, label: &str| {
        raw.parse::<usize>()
            .map_err(|_| format!("{label} must be a nonnegative frame address"))
    };
    frame_arithmetic_read(
        &value,
        parse_address(args[2], "left width")?,
        parse_address(args[3], "left group")?,
        args[4],
        parse_address(args[5], "right width")?,
        parse_address(args[6], "right group")?,
    )
}

/// Turn a prime-divisor witness into an exact factor-pair read and independently
/// check whether the cofactor is prime within the same sieve aperture.
pub fn factor_pair_read(
    value: &Nat,
    sieve: &PrimeSieveRead,
    width: usize,
) -> Result<Option<FactorPairRead>, String> {
    let Some(factor) = sieve.divisor.as_ref() else {
        return Ok(None);
    };
    let Some((cofactor, remainder)) = value.div_rem(factor) else {
        return Err("zero divisor in factor-pair closure".to_string());
    };
    let nontrivial = factor.cmp_nat(&Nat::one()) == Ordering::Greater
        && cofactor.cmp_nat(&Nat::one()) == Ordering::Greater;
    let product_closed = nontrivial && factor.mul(&cofactor) == *value && remainder.is_zero();
    if !product_closed {
        return Ok(Some(FactorPairRead {
            factor: factor.clone(),
            cofactor,
            product_closed,
            semiprime_closed: Some(false),
        }));
    }

    let cofactor_is_prime = if cofactor == Nat::from_u64(2) {
        Some(true)
    } else if !cofactor.bit(0) {
        Some(false)
    } else {
        let sqrt_aperture_width = cofactor.bits_le().len().div_ceil(2).max(2);
        let cofactor_width = width.min(sqrt_aperture_width);
        let cofactor_sieve = prime_sieve_read(&cofactor, cofactor_width)?;
        match cofactor_sieve.divisor.as_ref() {
            Some(witness) => Some(witness == &cofactor),
            None => cofactor_sieve.lower_bound.as_ref().and_then(|bound| {
                let bound_squared = bound.mul(bound);
                (bound_squared.cmp_nat(&cofactor) != Ordering::Less).then_some(true)
            }),
        }
    };
    Ok(Some(FactorPairRead {
        factor: factor.clone(),
        cofactor,
        product_closed,
        semiprime_closed: cofactor_is_prime,
    }))
}
