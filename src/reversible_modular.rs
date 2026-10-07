//! Stream reversible modular arithmetic without enumerating residue states.
//!
//! Gates describe Boolean controls on logical anyon qubits. Lowering these
//! gates to calibrated braids and supplying a carrier readout remain separate
//! execution requirements. This module does not manufacture a phase sample.
use alloc::string::String;
use alloc::vec::Vec;
use num_bigint::{BigInt, BigUint, Sign};
use num_traits::{One, Zero};

/// A reversible X controlled by conjunction of positive or negative literals.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControlledX {
    pub controls: Vec<(usize, bool)>,
    pub target: usize,
}

/// Reversible arithmetic boundaries before clean carry ancillas are lowered.
/// A shared-branch backend can execute each complete split/fuse operation
/// without materializing its elementary workspace history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NestedOperation {
    Toggle(ControlledX),
    ModularAdd { register: Vec<usize>, digit: Vec<usize>, value: BigUint, modulus: BigUint, controls: Vec<(usize, bool)> },
    Add { register: Vec<usize>, value: BigUint, controls: Vec<(usize, bool)> },
    Compare { register: Vec<usize>, value: BigUint, controls: Vec<(usize, bool)>, flag: usize },
}

impl NestedOperation {
    /// Adjacent return map on the same register and controls.
    pub fn inverse(&self) -> Self {
        match self {
            Self::Toggle(_) | Self::Compare { .. } => self.clone(),
            Self::Add { register, value, controls } => {
                let radix = BigUint::one() << register.len();
                let value = (radix.clone() - value % &radix) % radix;
                Self::Add { register: register.clone(), value, controls: controls.clone() }
            }
            Self::ModularAdd { register, digit, value, modulus, controls } => {
                let value = (modulus - value % modulus) % modulus;
                Self::ModularAdd { register: register.clone(), digit: digit.clone(), value,
                    modulus: modulus.clone(), controls: controls.clone() }
            }
        }
    }
}

/// Elementary reversible gates with at most two controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementaryGate {
    X(usize),
    Cnot {
        control: usize,
        target: usize,
    },
    Toffoli {
        first: usize,
        second: usize,
        target: usize,
    },
}

// Internal arithmetic generates distinct controls and disjoint targets. The
// scratch register is allocated outside its entire logical register layout.
fn lower<F>(operation: &ControlledX, scratch: &[usize], emit: &mut F) -> Result<(), String>
where
    F: FnMut(ElementaryGate) -> Result<(), String>,
{
    let count = operation.controls.len();
    if scratch.len() < count.saturating_sub(2) {
        return Err("insufficient clean workspace for controlled X".into());
    }
    for &(q, positive) in &operation.controls {
        if !positive {
            emit(ElementaryGate::X(q))?;
        }
    }
    match count {
        0 => emit(ElementaryGate::X(operation.target))?,
        1 => emit(ElementaryGate::Cnot {
            control: operation.controls[0].0,
            target: operation.target,
        })?,
        2 => emit(ElementaryGate::Toffoli {
            first: operation.controls[0].0,
            second: operation.controls[1].0,
            target: operation.target,
        })?,
        _ => {
            emit(ElementaryGate::Toffoli {
                first: operation.controls[0].0,
                second: operation.controls[1].0,
                target: scratch[0],
            })?;
            for i in 1..count - 2 {
                emit(ElementaryGate::Toffoli {
                    first: scratch[i - 1],
                    second: operation.controls[i + 1].0,
                    target: scratch[i],
                })?;
            }
            emit(ElementaryGate::Toffoli {
                first: scratch[count - 3],
                second: operation.controls[count - 1].0,
                target: operation.target,
            })?;
            for i in (1..count - 2).rev() {
                emit(ElementaryGate::Toffoli {
                    first: scratch[i - 1],
                    second: operation.controls[i + 1].0,
                    target: scratch[i],
                })?;
            }
            emit(ElementaryGate::Toffoli {
                first: operation.controls[0].0,
                second: operation.controls[1].0,
                target: scratch[0],
            })?;
        }
    }
    for &(q, positive) in operation.controls.iter().rev() {
        if !positive {
            emit(ElementaryGate::X(q))?;
        }
    }
    Ok(())
}

fn gate<F>(controls: &[(usize, bool)], target: usize, emit: &mut F) -> Result<(), String>
where
    F: FnMut(ControlledX) -> Result<(), String>,
{
    emit(ControlledX {
        controls: controls.to_vec(),
        target,
    })
}

/// Add a constant modulo 2^width. Descending carries preserve each original
/// lower-bit condition until its target is toggled.
fn add<F>(
    register: &[usize],
    value: &BigUint,
    controls: &[(usize, bool)],
    emit: &mut F,
) -> Result<(), String>
where
    F: FnMut(ControlledX) -> Result<(), String>,
{
    for bit in 0..register.len() {
        if !value.bit(bit as u64) {
            continue;
        }
        for higher in (bit + 1..register.len()).rev() {
            let mut carry = controls.to_vec();
            carry.extend(register[bit..higher].iter().map(|&q| (q, true)));
            gate(&carry, register[higher], emit)?;
        }
        gate(controls, register[bit], emit)?;
    }
    Ok(())
}

/// Compute carries from the original register, apply the controlled sum, then
/// erase carries from high to low while briefly restoring each original bit.
fn ripple_add<F>(
    register: &[usize],
    value: &BigUint,
    controls: &[(usize, bool)],
    carries: &[usize],
    emit: &mut F,
) -> Result<(), String>
where
    F: FnMut(ControlledX) -> Result<(), String>,
{
    if carries.len() < register.len().saturating_sub(1) {
        return Err("insufficient ripple carry workspace".into());
    }
    let carry_gate = |i: usize, emit: &mut F| -> Result<(), String> {
        if i == 0 {
            if value.bit(0) {
                gate(&[(register[0], true)], carries[0], emit)?;
            }
        } else {
            if value.bit(i as u64) {
                gate(&[(register[i], true)], carries[i], emit)?;
                gate(&[(carries[i - 1], true)], carries[i], emit)?;
            }
            gate(
                &[(register[i], true), (carries[i - 1], true)],
                carries[i],
                emit,
            )?;
        }
        Ok(())
    };
    let sum_gate = |i: usize, emit: &mut F| -> Result<(), String> {
        if value.bit(i as u64) {
            gate(controls, register[i], emit)?;
        }
        if i > 0 {
            let mut condition = controls.to_vec();
            condition.push((carries[i - 1], true));
            gate(&condition, register[i], emit)?;
        }
        Ok(())
    };
    for i in 0..register.len().saturating_sub(1) {
        carry_gate(i, emit)?;
    }
    for i in 0..register.len() {
        sum_gate(i, emit)?;
    }
    for i in (0..register.len().saturating_sub(1)).rev() {
        sum_gate(i, emit)?;
        carry_gate(i, emit)?;
        sum_gate(i, emit)?;
    }
    Ok(())
}

/// The first differing bit partitions y < value into disjoint conditions.
fn less_than<F>(
    register: &[usize],
    value: &BigUint,
    controls: &[(usize, bool)],
    flag: usize,
    emit: &mut F,
) -> Result<(), String>
where
    F: FnMut(ControlledX) -> Result<(), String>,
{
    for bit in 0..register.len() {
        if !value.bit(bit as u64) {
            continue;
        }
        let mut condition = controls.to_vec();
        condition.push((register[bit], false));
        condition.extend((bit + 1..register.len()).map(|i| (register[i], value.bit(i as u64))));
        gate(&condition, flag, emit)?;
    }
    Ok(())
}

/// Modular outputs and constants have a zero extension bit. Compare their
/// remaining bits by computing and then erasing the subtraction borrow chain.
fn ripple_less<F>(
    register: &[usize],
    value: &BigUint,
    controls: &[(usize, bool)],
    flag: usize,
    carries: &[usize],
    emit: &mut F,
) -> Result<(), String>
where
    F: FnMut(ControlledX) -> Result<(), String>,
{
    let data = &register[..register.len() - 1];
    if carries.len() < data.len() {
        return Err("insufficient comparison workspace".into());
    }
    let borrow_gate = |i: usize, emit: &mut F| -> Result<(), String> {
        if i == 0 {
            if value.bit(0) {
                gate(&[(data[0], false)], carries[0], emit)?;
            }
        } else {
            if value.bit(i as u64) {
                gate(&[(data[i], false)], carries[i], emit)?;
                gate(&[(carries[i - 1], true)], carries[i], emit)?;
            }
            gate(
                &[(data[i], false), (carries[i - 1], true)],
                carries[i],
                emit,
            )?;
        }
        Ok(())
    };
    for i in 0..data.len() {
        borrow_gate(i, emit)?;
    }
    let mut condition = controls.to_vec();
    condition.push((carries[data.len() - 1], true));
    gate(&condition, flag, emit)?;
    for i in (0..data.len()).rev() {
        borrow_gate(i, emit)?;
    }
    Ok(())
}

/// On y < N, add c modulo N. The extra high bit and flag both return to zero.
fn add_mod<F>(
    register: &[usize],
    c: &BigUint,
    n: &BigUint,
    controls: &[(usize, bool)],
    flag: usize,
    emit: &mut F,
) -> Result<(), String>
where
    F: FnMut(NestedOperation) -> Result<(), String>,
{
    if c.is_zero() {
        return Ok(());
    }
    let radix = BigUint::one() << register.len();
    emit(NestedOperation::Add { register: register.to_vec(), value: c.clone(), controls: controls.to_vec() })?;
    emit(NestedOperation::Add { register: register.to_vec(), value: &radix - n, controls: controls.to_vec() })?;
    let mut sign = controls.to_vec();
    sign.push((*register.last().ok_or("empty modular register")?, true));
    emit(NestedOperation::Toggle(ControlledX { controls: sign, target: flag }))?;
    emit(NestedOperation::Add { register: register.to_vec(), value: n.clone(), controls: alloc::vec![(flag,true)] })?;
    // Underflow means no wrap; the final result < c means wrap. These are
    // complementary on the valid input subspace, so erase the borrow flag.
    emit(NestedOperation::Toggle(ControlledX { controls: controls.to_vec(), target: flag }))?;
    emit(NestedOperation::Compare { register: register.to_vec(), value: c.clone(), controls: controls.to_vec(), flag })
}

fn inverse(value: &BigUint, n: &BigUint) -> Result<BigUint, String> {
    let modulus = BigInt::from_biguint(Sign::Plus, n.clone());
    let (mut old_r, mut r) = (
        modulus.clone(),
        BigInt::from_biguint(Sign::Plus, value.clone()),
    );
    let (mut old_t, mut t) = (BigInt::zero(), BigInt::one());
    while !r.is_zero() {
        let quotient = &old_r / &r;
        let next_r = &old_r - &quotient * &r;
        let next_t = &old_t - &quotient * &t;
        old_r = r;
        r = next_r;
        old_t = t;
        t = next_t;
    }
    if old_r != BigInt::one() {
        return Err("modular multiplier must be coprime to N".into());
    }
    let result = ((old_t % &modulus) + &modulus) % &modulus;
    result
        .to_biguint()
        .ok_or_else(|| "negative modular inverse".into())
}

/// Logical register layout for a recycled control and reversible multiply.
/// Requires 2n+3 qubits including the control and clean arithmetic workspace.
/// The workspace is returned clean after each controlled multiplication.
pub struct ModularMultiply {
    n: BigUint,
    width: usize,
}

impl ModularMultiply {
    pub fn new(n: &BigUint) -> Result<Self, String> {
        if n.bits() < 128 {
            return Err("modular factorization source must be at least 128 bits".into());
        }
        let width = usize::try_from(n.bits()).map_err(|_| "source width exceeds host indexing")?;
        width
            .checked_mul(3)
            .and_then(|w| w.checked_add(4))
            .ok_or("qubit layout overflow")?;
        Ok(Self {
            n: n.clone(),
            width,
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }
    pub fn qubits(&self) -> usize {
        2 * self.width + 3
    }

    /// Includes n ripple carry ancillas and one control-lowering ancilla.
    pub fn elementary_qubits(&self) -> usize {
        3 * self.width + 4
    }

    /// Reserve wires zero and one for the two components of one ququart.
    /// Remaining wires share the same work register in both controlled powers.
    pub fn emit_ququart_elementary<F>(&self, multiplier: &BigUint, lane: usize, mut emit: F) -> Result<(), String>
    where F: FnMut(ElementaryGate) -> Result<(), String> {
        if lane > 1 { return Err("ququart control lane must be zero or one".into()); }
        let wire=|q| if q==0 {lane} else {q+1};
        self.emit_elementary(multiplier, |gate| emit(match gate {
            ElementaryGate::X(q)=>ElementaryGate::X(wire(q)),
            ElementaryGate::Cnot { control,target }=>ElementaryGate::Cnot { control:wire(control),target:wire(target) },
            ElementaryGate::Toffoli { first,second,target }=>ElementaryGate::Toffoli { first:wire(first),second:wire(second),target:wire(target) },
        }))
    }

    /// Stream X/CNOT/Toffoli gates. The extra ancillas return to zero after
    /// each logical operation, including operations with negative controls.
    /// Consumers must abort execution if emitting any gate returns an error.
    pub fn emit_elementary<F>(&self, multiplier: &BigUint, mut emit: F) -> Result<(), String>
    where
        F: FnMut(ElementaryGate) -> Result<(), String>,
    {
        let scratch: Vec<_> = (self.qubits()..self.elementary_qubits()).collect();
        self.emit_with_ops(
            multiplier,
            |operation| {
                // Ripple sums and comparisons keep n carries live. Gates with
                // three controls use the final free ancilla.
                let workspace = if operation.controls.len() <= 3 {
                    &scratch[scratch.len() - 1..]
                } else {
                    &scratch[..]
                };
                lower(&operation, workspace, &mut emit)
            },
            |register, value, controls, output| {
                ripple_add(register, value, controls, &scratch[..self.width], output)
            },
            |register, value, controls, flag, output| {
                ripple_less(
                    register,
                    value,
                    controls,
                    flag,
                    &scratch[..self.width],
                    output,
                )
            },
        )
    }

    /// Layout: control 0, source 1..=n, workspace n+1..=2n+1,
    /// borrow flag 2n+2. Source must be < N and workspace initially zero.
    /// Emits gates immediately; no state vector or complete gate list exists.
    pub fn emit<F>(&self, multiplier: &BigUint, emit: F) -> Result<(), String>
    where
        F: FnMut(ControlledX) -> Result<(), String>,
    {
        self.emit_with_ops(multiplier, emit, add, less_than)
    }

    pub fn emit_ququart_nested<F>(
        &self, multiplier: &BigUint, lane: usize, emit: F,
    ) -> Result<(), String>
    where F: FnMut(NestedOperation) -> Result<(), String> {
        self.emit_ququart_nested_radix(multiplier,lane,1,emit)
    }

    pub fn emit_ququart_nested_radix<F>(&self, multiplier: &BigUint, lane: usize, digit_bits: usize, mut emit: F) -> Result<(),String>
    where F: FnMut(NestedOperation) -> Result<(),String> {
        if digit_bits == 0 { return Err("empty work radix digit".into()); }
        if lane > 1 { return Err("ququart control lane must be zero or one".into()); }
        let wire = |q| if q == 0 { lane } else { q + 1 };
        let output = |operation: NestedOperation| {
            let controls = |items: Vec<(usize, bool)>| items.into_iter()
                .map(|(q, value)| (wire(q), value)).collect();
            let register = |items: Vec<usize>| items.into_iter().map(wire).collect();
            emit(match operation {
                NestedOperation::ModularAdd { register: r, digit, value, modulus, controls: c } => NestedOperation::ModularAdd {
                    register: register(r), digit: register(digit), value, modulus, controls: controls(c),
                },
                NestedOperation::Toggle(gate) => NestedOperation::Toggle(ControlledX {
                    target: wire(gate.target), controls: controls(gate.controls),
                }),
                NestedOperation::Add { register: r, value, controls: c } => NestedOperation::Add {
                    register: register(r), value, controls: controls(c),
                },
                NestedOperation::Compare { register: r, value, controls: c, flag } => NestedOperation::Compare {
                    register: register(r), value, controls: controls(c), flag: wire(flag),
                },
            })
        };
        // Keep each complete modular translation at the shared boundary;
        // elementary lowering retains the borrow-flag shell on the same rail.
        self.emit_nested(multiplier, true, digit_bits, output)
    }

    fn emit_with_ops<F, A, C>(
        &self,
        multiplier: &BigUint,
        mut emit: F,
        mut addition: A,
        mut comparison: C,
    ) -> Result<(), String>
    where
        F: FnMut(ControlledX) -> Result<(), String>,
        A: FnMut(&[usize], &BigUint, &[(usize, bool)], &mut F) -> Result<(), String>,
        C: FnMut(&[usize], &BigUint, &[(usize, bool)], usize, &mut F) -> Result<(), String>,
    {
        self.emit_nested(multiplier, false, 1, |operation| match operation {
            NestedOperation::ModularAdd { .. } => Err("modular boundary reached elementary lowering".into()),
            NestedOperation::Toggle(gate) => emit(gate),
            NestedOperation::Add { register, value, controls } =>
                addition(&register, &value, &controls, &mut emit),
            NestedOperation::Compare { register, value, controls, flag } =>
                comparison(&register, &value, &controls, flag, &mut emit),
        })
    }

    fn emit_nested<F>(&self, multiplier: &BigUint, whole_modular: bool, digit_bits: usize, mut emit: F) -> Result<(), String>
    where F: FnMut(NestedOperation) -> Result<(), String> {
        let multiplier = multiplier % &self.n;
        if multiplier.is_one() {
            return Ok(());
        }
        let inverse = inverse(&multiplier, &self.n)?;
        let source: Vec<_> = (1..=self.width).collect();
        let workspace: Vec<_> = (self.width + 1..=2 * self.width + 1).collect();
        let flag = 2 * self.width + 2;
        let mut power = multiplier;
        for group in source.chunks(digit_bits) {
            if whole_modular {
                emit(NestedOperation::ModularAdd { register: workspace.clone(), digit: group.to_vec(),
                    value: power.clone(), modulus: self.n.clone(), controls: alloc::vec![(0,true)] })?;
            } else {
                add_mod(&workspace,&power,&self.n,&[(0,true),(group[0],true)],flag,&mut emit)?;
            }
            power = (&power << group.len()) % &self.n;
        }
        for (&x, &y) in source.iter().zip(&workspace) {
            for (control, target) in [(x,y), (y,x), (x,y)] {
                emit(NestedOperation::Toggle(ControlledX {
                    controls: alloc::vec![(0,true),(control,true)], target,
                }))?;
            }
        }
        let mut power = inverse;
        for group in source.chunks(digit_bits) {
            let negative = if power.is_zero() { BigUint::zero() } else { &self.n - &power };
            if whole_modular {
                emit(NestedOperation::ModularAdd { register: workspace.clone(), digit: group.to_vec(),
                    value: negative, modulus: self.n.clone(), controls: alloc::vec![(0,true)] })?;
            } else {
                add_mod(&workspace,&negative,&self.n,&[(0,true),(group[0],true)],flag,&mut emit)?;
            }
            power = (&power << group.len()) % &self.n;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reversible_modular_elementary_multiply_128_bit_semiprime() {
        let n = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        assert_eq!(n.bits(), 128);
        let circuit = ModularMultiply::new(&n).unwrap();
        let x = &n - BigUint::from(2u8);
        let multiplier = BigUint::from(2u8);
        for enabled in [false, true] {
            let mut bits = alloc::vec![false; circuit.elementary_qubits()];
            let mut gate_count = 0usize;
            bits[0] = enabled;
            for i in 0..circuit.width() {
                bits[i + 1] = x.bit(i as u64);
            }
            circuit
                .emit_elementary(&multiplier, |operation| {
                    gate_count += 1;
                    let (target, apply) = match operation {
                        ElementaryGate::X(target) => (target, true),
                        ElementaryGate::Cnot { control, target } => (target, bits[control]),
                        ElementaryGate::Toffoli {
                            first,
                            second,
                            target,
                        } => (target, bits[first] && bits[second]),
                    };
                    if apply {
                        bits[target] = !bits[target];
                    }
                    Ok(())
                })
                .unwrap();
            let mut result = BigUint::zero();
            for i in 0..circuit.width() {
                if bits[i + 1] {
                    result |= BigUint::one() << i;
                }
            }
            assert_eq!(
                result,
                if enabled {
                    (&x * &multiplier) % &n
                } else {
                    x.clone()
                }
            );
            assert_eq!(bits[0], enabled);
            assert!(bits[circuit.width() + 1..].iter().all(|&bit| !bit));
            assert!(gate_count <= 512 * circuit.width() * circuit.width());
            std::println!("128-bit modular multiply: control={enabled}, elementary gates={gate_count}, qubits={}", circuit.elementary_qubits());
        }
    }

    #[test]
    fn reversible_modular_controlled_multiply_128_bit_semiprime() {
        // Independent fixture: 16925480323643806501 * 17526877587580975651.
        // Only N enters the circuit constructor. This checks arithmetic, not
        // factor extraction or phase measurement.
        let n = BigUint::parse_bytes(b"296650821743515430283258444261036507151", 10).unwrap();
        assert_eq!(n.bits(), 128);
        let circuit = ModularMultiply::new(&n).unwrap();
        let x = &n - BigUint::from(2u8);
        let multiplier = BigUint::from(2u8);
        for enabled in [false, true] {
            let mut bits = alloc::vec![false; circuit.qubits()];
            bits[0] = enabled;
            for i in 0..circuit.width() {
                bits[i + 1] = x.bit(i as u64);
            }
            let mut gates = 0usize;
            circuit
                .emit(&multiplier, |operation| {
                    assert!(operation.target < bits.len());
                    assert!(operation
                        .controls
                        .iter()
                        .all(|&(q, _)| q != operation.target));
                    if operation
                        .controls
                        .iter()
                        .all(|&(q, value)| bits[q] == value)
                    {
                        bits[operation.target] = !bits[operation.target];
                    }
                    gates += 1;
                    Ok(())
                })
                .unwrap();
            let mut result = BigUint::zero();
            for i in 0..circuit.width() {
                if bits[i + 1] {
                    result |= BigUint::one() << i;
                }
            }
            assert_eq!(
                result,
                if enabled {
                    (&x * &multiplier) % &n
                } else {
                    x.clone()
                }
            );
            assert_eq!(bits[0], enabled);
            assert!(bits[circuit.width() + 1..].iter().all(|&bit| !bit));
            assert!(gates > 0);
        }
    }

    #[test]
    fn elementary_gate_stream_stays_width_bounded_on_unstructured_semiprimes() {
        let cases = [
            (128usize, "296650821743515430283258444261036507151"),
            (192usize, "3448910520600450090963625683018141073856509123385137086401"),
            (256usize, "74190557381655886253556359637698917958655348096397072330011641798610025580603"),
            (512usize, "9969906765783880964144656106420939635433526042367203781554613194134456045788303190678018270287475265649670195674743890452325667572153673384288330674180821"),
            (1024usize, "151777019658254876857817633777310500067543531862690952628150066934774348856446724909442250322963432373284529849953977441737417349575905634097035061187457141916353707279440643579659068527565844333535400729258064430177333685201919732761254826385161013978682394914675587203065373750392052441296616389598208643077"),
            // RSA-2048 challenge modulus, generated from two large prime factors.
            (2048usize, concat!(
                "2519590847565789349402718324004839857142928212620403202777713783604366202070",
                "7595556264018525880784406918290641249515082189298559149176184502808489120072",
                "8449926873928072877767359714183472702618963750149718246911650776133798590957",
                "0009733045974880842840179742910064245869181719511874612151517265463228221686",
                "9987549182422433637259085141865462043576798423387184774447920739934236584823",
                "8242811981638150106748104516603773060562016196762561338441436038339044149526",
                "3443219011465754445417842402092461651572335077870774981712577246796292638635",
                "6373289912154831438167899885040445364023527381951378636564391212010397122822",
                "120720357"
            )),
        ];
        for (expected_width, source) in cases {
            let n = BigUint::parse_bytes(source.as_bytes(), 10).unwrap();
            assert_eq!(n.bits() as usize, expected_width);
            let circuit = ModularMultiply::new(&n).unwrap();
            let width = expected_width as u64;
            assert_eq!(circuit.elementary_qubits(), 3 * expected_width + 4);
            for (label, multiplier) in [("two", BigUint::from(2u8)), ("dense", &n - BigUint::one())] {
                let mut gate_count = 0u64;
                let mut max_wire = 0usize;
                circuit.emit_elementary(&multiplier, |gate| {
                    gate_count += 1;
                    let wires: [usize; 3] = match gate {
                        ElementaryGate::X(q) => [q, q, q],
                        ElementaryGate::Cnot { control, target } => [control, target, target],
                        ElementaryGate::Toffoli { first, second, target } => [first, second, target],
                    };
                    max_wire = max_wire.max(*wires.iter().max().unwrap());
                    Ok(())
                }).unwrap();
                assert!(gate_count > 0);
                assert!(gate_count <= 96 * width * width, "{label} multiplier exceeded quadratic bound at {expected_width} bits");
                assert!(max_wire < circuit.elementary_qubits());
                std::println!("source_bits={expected_width} multiplier={label} streamed_gates={gate_count} allocated_qubits={}", circuit.elementary_qubits());
            }
        }
    }
}
