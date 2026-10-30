//! Resident QFT circuit. Depth follows CIRCUIT_LEVELS at module build time.
//! Wiring is prepared once; signals
//! propagate through fixed buffers on each activation. Feedback is sampled
//! from the preceding activation, so the return path has one activation delay.
use crate::membrane_complex::Complex;

const fn configured_levels() -> usize {
    let text = match option_env!("CIRCUIT_LEVELS") { Some(s) => s, None => "6" };
    let bytes = text.as_bytes();
    assert!(!bytes.is_empty());
    let mut value = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        assert!(bytes[i] >= b'0' && bytes[i] <= b'9');
        value = value * 10 + (bytes[i]-b'0') as usize;
        i += 1;
    }
    assert!(value > 0 && value < usize::BITS as usize);
    value
}
pub const LEVELS: usize = configured_levels();
pub const PORTS: usize = 1 << LEVELS;
const BUTTERFLIES: usize = PORTS / 2 * LEVELS;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Connection { a: usize, b: usize, sum: usize, difference: usize, phase: usize }
const EMPTY: Connection = Connection { a: 0, b: 0, sum: 0, difference: 0, phase: 0 };

pub struct QftCircuit {
    connections: [Connection; BUTTERFLIES],
    phases: [Complex; PORTS / 2],
    inputs: [usize; PORTS],
    signals: [Complex; PORTS * (LEVELS + 1)],
    ready: bool,
    scale: f64,
}

impl QftCircuit {
    pub const fn empty() -> Self {
        let mut connections = [EMPTY; BUTTERFLIES];
        let mut inputs = [0; PORTS];
        let mut j = 0usize;
        while j < PORTS { inputs[j] = j.reverse_bits() >> (usize::BITS - LEVELS as u32); j += 1; }
        let mut next = 0;
        let mut level = 0;
        while level < LEVELS {
            let width = 2 << level;
            let mut block = 0;
            while block < PORTS {
                let mut j = 0;
                while j < width/2 {
                    let a = block+j;
                    let b = a+width/2;
                    connections[next] = Connection { a: level*PORTS+a, b: level*PORTS+b,
                        sum: (level+1)*PORTS+a, difference: (level+1)*PORTS+b, phase: j*PORTS/width };
                    next += 1;
                    j += 1;
                }
                block += width;
            }
            level += 1;
        }
        Self { connections, phases: [Complex::zero(); PORTS/2],
            inputs, signals: [Complex::zero(); PORTS*(LEVELS+1)], ready: false, scale: 0.0 }
    }

    pub fn prepare(&mut self) {
        self.scale = 1.0 / (PORTS as f64).sqrt();
        for j in 0..PORTS/2 {
            let angle = 2.0 * core::f64::consts::PI * j as f64 / PORTS as f64;
            self.phases[j] = Complex::new(angle.cos(), angle.sin());
        }
        self.signals.fill(Complex::zero());
        self.ready = true;
    }

    /// Gate j modulo 64 selects a unit input, or the previous normalized output j when
    /// feedback is enabled. Closed gates inject zero. Every activation overwrites
    /// every signal slot, including the zero-input case. No allocation or wiring.
    pub fn activate(&mut self, gates: u64, feedback: bool) -> Result<(), &'static str> {
        if !self.ready { return Err("prepare the circuit before activation"); }
        for j in 0..PORTS {
            self.signals[self.inputs[j]] = if gates >> (j % 64) & 1 == 0 { Complex::zero() }
                else if feedback { self.signals[LEVELS*PORTS+j].scale(self.scale) }
                else { Complex::one() };
        }
        for wire in &self.connections {
            let even = self.signals[wire.a];
            let odd = self.signals[wire.b] * self.phases[wire.phase];
            self.signals[wire.sum] = even + odd;
            self.signals[wire.difference] = even - odd;
        }
        Ok(())
    }

    pub fn output(&self, port: usize) -> Complex { self.signals[LEVELS*PORTS+port].scale(self.scale) }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn direct(input: &[Complex; PORTS]) -> [Complex; PORTS] {
        core::array::from_fn(|k| {
            let mut sum = Complex::zero();
            for (j, value) in input.iter().enumerate() {
                let angle = 2.0*core::f64::consts::PI*(j*k) as f64/PORTS as f64;
                sum = sum + *value * Complex::new(angle.cos(), angle.sin());
            }
            sum.scale(1.0 / (PORTS as f64).sqrt())
        })
    }
    #[test]
    fn resident_gates_and_feedback_match_direct_transform() {
        let mut circuit = QftCircuit::empty();
        assert!(circuit.activate(1, false).is_err());
        circuit.prepare();
        let wiring = circuit.connections;
        let phases = circuit.phases;
        let mut previous = [Complex::zero(); PORTS];
        for (gates, feedback) in [(0, false), (1, false), (3, false), (0x0123456789abcdef, false),
            (u64::MAX, true), (0x5555555555555555, true), (0, false), (1 << 63, false), (1, false)] {
            let input = core::array::from_fn(|j| if gates >> (j % 64) & 1 == 0 { Complex::zero() }
                else if feedback { previous[j] } else { Complex::one() });
            previous = direct(&input);
            circuit.activate(gates, feedback).unwrap();
            for (j, expected) in previous.iter().enumerate() {
                let got = circuit.output(j);
                assert!((got.re-expected.re).abs() < 1e-10 && (got.im-expected.im).abs() < 1e-10, "port={j}");
            }
            assert_eq!(circuit.connections, wiring);
            assert_eq!(circuit.phases, phases);
        }
    }
}
