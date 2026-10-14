//! Reversible factor-specific phase: |p,q> -> (-1)^[p*q=N,p>1,q>1]|p,q>.
//! The source and every Boolean control value are resident IMASM numerals.
//! Gates stream through one shared product, row and carry workspace. The
//! ascending rail reverses the descending rail after the phase operation.
use crate::vox::{EVALF, EVALT};
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Control {
    pub cell: usize,
    pub value: char,
}

/// A backend must implement controlled X and controlled sign reversal as
/// coherent operations. These operations are each their own inverse.
pub trait GateSink {
    fn toggle(&mut self, controls: &[Control], target: usize) -> Result<(), &'static str>;
    fn phase_flip(&mut self, controls: &[Control]) -> Result<(), &'static str>;
    fn checkpoint(&mut self) -> Result<(), &'static str> { Ok(()) }
}

/// Explicit source-support intersection for a glut relation. This operation
/// is conditional support selection, separate from the coherent gate action.
pub trait FactorSupportSink: GateSink {
    fn retain_literals(&mut self, controls: &[Control]) -> Result<(), &'static str>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    pub p: core::ops::Range<usize>,
    pub q: core::ops::Range<usize>,
    pub product: core::ops::Range<usize>,
    pub row: core::ops::Range<usize>,
    pub carry: usize,
    pub nontrivial_p: usize,
    pub nontrivial_q: usize,
    pub cells: usize,
}

/// Preparation is linear in source width; no candidate pairs or gate sequence
/// are retained. Lowering emits polynomially many reversible Boolean gates.
/// Applying a phase oracle does not itself perform factor measurement.
pub struct FactorPhaseOracle<'a> {
    n: &'a [char],
    layout: Layout,
}
impl<'a> FactorPhaseOracle<'a> {
    pub fn from_n(n: &'a [char]) -> Result<Self, &'static str> {
        if n.is_empty() || n.iter().any(|&c| c != EVALT && c != EVALF) {
            return Err("factor phase source is not an IMASM numeral");
        }
        let width = n.iter().rposition(|&c| c == EVALF).map_or(0, |i| i + 1);
        if width < 2 {
            return Err("factor phase source must exceed one");
        }
        let two = width
            .checked_mul(2)
            .ok_or("factor register address overflow")?;
        let four = width
            .checked_mul(4)
            .ok_or("factor register address overflow")?;
        let six = width
            .checked_mul(6)
            .ok_or("factor register address overflow")?;
        let cells = six
            .checked_add(3)
            .ok_or("factor register address overflow")?;
        Ok(Self {
            n,
            layout: Layout {
                p: 0..width,
                q: width..two,
                product: two..four,
                row: four..six,
                carry: six,
                nontrivial_p: six + 1,
                nontrivial_q: six + 2,
                cells,
            },
        })
    }
    pub fn n(&self) -> &[char] {
        self.n
    }
    pub fn layout(&self) -> &Layout {
        &self.layout
    }
    /// Keep the same source while binding only low factor and work columns.
    pub fn product_prefix(&self, columns: usize) -> Result<Self, &'static str> {
        if columns == 0 || columns > self.layout.p.len() {
            return Err("product prefix exceeds its source layout");
        }
        Ok(Self { n: self.n, layout: Layout {
            p: 0..columns, q: columns..2*columns,
            product: 2*columns..4*columns, row: 4*columns..6*columns,
            carry: 6*columns, nontrivial_p: 6*columns+1,
            nontrivial_q: 6*columns+2, cells: 6*columns+3,
        } })
    }
    fn cx<S: GateSink>(sink: &mut S, control: usize, target: usize) -> Result<(), &'static str> {
        sink.toggle(
            &[Control {
                cell: control,
                value: EVALF,
            }],
            target,
        )
    }
    fn ccx<S: GateSink>(
        sink: &mut S,
        a: usize,
        b: usize,
        target: usize,
    ) -> Result<(), &'static str> {
        sink.toggle(
            &[
                Control {
                    cell: a,
                    value: EVALF,
                },
                Control {
                    cell: b,
                    value: EVALF,
                },
            ],
            target,
        )
    }
    // MAJ transports the carry into a; UMA restores a and the incoming carry
    // while leaving the sum in b. The reverse rail reverses the actual gates.
    fn majority<S: GateSink>(
        sink: &mut S,
        a: usize,
        b: usize,
        c: usize,
        reverse: bool,
    ) -> Result<(), &'static str> {
        if reverse {
            Self::ccx(sink, c, b, a)?;
            Self::cx(sink, a, c)?;
            Self::cx(sink, a, b)
        } else {
            Self::cx(sink, a, b)?;
            Self::cx(sink, a, c)?;
            Self::ccx(sink, c, b, a)
        }
    }
    fn unmajority<S: GateSink>(
        sink: &mut S,
        a: usize,
        b: usize,
        c: usize,
        reverse: bool,
    ) -> Result<(), &'static str> {
        if reverse {
            Self::cx(sink, c, b)?;
            Self::cx(sink, a, c)?;
            Self::ccx(sink, c, b, a)
        } else {
            Self::ccx(sink, c, b, a)?;
            Self::cx(sink, a, c)?;
            Self::cx(sink, c, b)
        }
    }
    fn add_row<S: GateSink>(&self, sink: &mut S, reverse: bool) -> Result<(), &'static str> {
        let l = &self.layout;
        let width = l.product.len();
        let at = |i| {
            (
                l.row.start + i,
                l.product.start + i,
                if i == 0 { l.carry } else { l.row.start + i - 1 },
            )
        };
        if reverse {
            for i in 0..width {
                let (a, b, c) = at(i);
                Self::unmajority(sink, a, b, c, true)?;
            }
            for i in (0..width).rev() {
                let (a, b, c) = at(i);
                Self::majority(sink, a, b, c, true)?;
            }
        } else {
            for i in 0..width {
                let (a, b, c) = at(i);
                Self::majority(sink, a, b, c, false)?;
            }
            for i in (0..width).rev() {
                let (a, b, c) = at(i);
                Self::unmajority(sink, a, b, c, false)?;
            }
        }
        Ok(())
    }
    fn row<S: GateSink>(&self, sink: &mut S, i: usize, reverse: bool) -> Result<(), &'static str> {
        let l = &self.layout;
        // Partial product cells are reused for each row and cleared immediately.
        for j in 0..l.q.len() {
            Self::ccx(sink, l.p.start + i, l.q.start + j, l.row.start + i + j)?;
        }
        self.add_row(sink, reverse)?;
        for j in (0..l.q.len()).rev() {
            Self::ccx(sink, l.p.start + i, l.q.start + j, l.row.start + i + j)?;
        }
        sink.checkpoint()
    }
    fn nontrivial<S: GateSink>(
        &self,
        sink: &mut S,
        register: core::ops::Range<usize>,
        flag: usize,
        reverse: bool,
    ) -> Result<(), &'static str> {
        let zero_high: Vec<_> = (register.start + 1..register.end)
            .map(|cell| Control { cell, value: EVALT })
            .collect();
        if reverse {
            sink.toggle(&zero_high, flag)?;
            sink.toggle(&[], flag)
        } else {
            sink.toggle(&[], flag)?;
            sink.toggle(&zero_high, flag)
        }
    }
    /// Requires all workspace cells at zero. Each factor register is unchanged
    /// and every workspace cell returns to zero, including on marked branches.
    pub fn lower<S: GateSink>(&self, sink: &mut S) -> Result<(), &'static str> {
        self.lower_with_selection(sink, |_, _| Ok(()))
    }

    /// At row i, product bit i is final: every later row starts above it.
    /// Intersect that completed column with N before advancing the same
    /// factor/product/carry register. The final intersection includes properness.
    pub fn lower_selected<S: FactorSupportSink>(&self, sink: &mut S)
        -> Result<(), &'static str> {
        self.lower_with_selection(sink, |state, controls| state.retain_literals(controls))
    }

    /// Select only the requested low product columns and restore all workspace.
    /// Properness concerns complete factors and is deliberately deferred here.
    pub fn lower_selected_product_prefix<S: FactorSupportSink>(&self, sink: &mut S,
        columns: usize) -> Result<(), &'static str> {
        if columns == 0 || columns > self.layout.p.len() {
            return Err("product prefix exceeds its source layout");
        }
        for i in 0..columns {
            self.row(sink, i, false)?;
            sink.retain_literals(&[Control { cell: self.layout.product.start + i,
                value: self.n.get(i).copied().unwrap_or(EVALT) }])?;
        }
        for i in (0..columns).rev() { self.row(sink, i, true)?; }
        Ok(())
    }

    fn lower_with_selection<S: GateSink>(&self, sink: &mut S,
        mut select: impl FnMut(&mut S, &[Control]) -> Result<(), &'static str>)
        -> Result<(), &'static str> {
        let l = &self.layout;
        for i in 0..l.p.len() {
            self.row(sink, i, false)?;
            select(sink, &[Control { cell: l.product.start + i,
                value: self.n.get(i).copied().unwrap_or(EVALT) }])?;
        }
        self.nontrivial(sink, l.p.clone(), l.nontrivial_p, false)?;
        self.nontrivial(sink, l.q.clone(), l.nontrivial_q, false)?;
        let mut matched: Vec<_> = l
            .product
            .clone()
            .enumerate()
            .map(|(i, cell)| Control {
                cell,
                value: self.n.get(i).copied().unwrap_or(EVALT),
            })
            .collect();
        matched.push(Control {
            cell: l.nontrivial_p,
            value: EVALF,
        });
        matched.push(Control {
            cell: l.nontrivial_q,
            value: EVALF,
        });
        select(sink, &matched)?;
        sink.phase_flip(&matched)?;
        self.nontrivial(sink, l.q.clone(), l.nontrivial_q, true)?;
        self.nontrivial(sink, l.p.clone(), l.nontrivial_p, true)?;
        for i in (0..l.p.len()).rev() {
            self.row(sink, i, true)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    fn numeral(mut n: usize) -> Vec<char> {
        let mut out = Vec::new();
        while n > 0 {
            out.push(if n & 1 == 1 { EVALF } else { EVALT });
            n >>= 1;
        }
        if out.is_empty() {
            out.push(EVALT);
        }
        out
    }
    struct Basis {
        cells: Vec<char>,
        negative: bool,
        gates: usize,
    }
    impl GateSink for Basis {
        fn toggle(&mut self, controls: &[Control], target: usize) -> Result<(), &'static str> {
            assert!(controls.iter().all(|c| c.cell != target));
            if controls.iter().all(|c| self.cells[c.cell] == c.value) {
                self.cells[target] = if self.cells[target] == EVALT {
                    EVALF
                } else {
                    EVALT
                };
            }
            self.gates += 1;
            Ok(())
        }
        fn phase_flip(&mut self, controls: &[Control]) -> Result<(), &'static str> {
            self.negative ^= controls.iter().all(|c| self.cells[c.cell] == c.value);
            self.gates += 1;
            Ok(())
        }
    }
    struct SelectedBasis { basis: Basis, retained: bool }
    impl GateSink for SelectedBasis {
        fn toggle(&mut self, controls: &[Control], target: usize) -> Result<(), &'static str> {
            self.basis.toggle(controls, target)
        }
        fn phase_flip(&mut self, controls: &[Control]) -> Result<(), &'static str> {
            self.basis.phase_flip(controls)
        }
    }
    impl FactorSupportSink for SelectedBasis {
        fn retain_literals(&mut self, controls: &[Control]) -> Result<(), &'static str> {
            self.retained &= controls.iter().all(|c| self.basis.cells[c.cell] == c.value);
            Ok(())
        }
    }
    #[test]
    fn low_product_selection_matches_modular_product_and_restores_workspace() {
        for n in [17usize, 32, 91, 143] {
            let tape = numeral(n);
            let full = FactorPhaseOracle::from_n(&tape).unwrap();
            for columns in 1..=4 {
                let oracle = full.product_prefix(columns).unwrap();
                let l = oracle.layout();
                for p in 0..1usize << columns {
                    for q in 0..1usize << columns {
                        let mut state = SelectedBasis { basis: Basis {
                            cells: vec![EVALT; l.cells], negative: false, gates: 0,
                        }, retained: true };
                        for (range, value) in [(l.p.clone(), p), (l.q.clone(), q)] {
                            for (i, cell) in range.enumerate() {
                                state.basis.cells[cell] = if value >> i & 1 == 1 { EVALF } else { EVALT };
                            }
                        }
                        let before = state.basis.cells.clone();
                        oracle.lower_selected_product_prefix(&mut state, columns).unwrap();
                        assert_eq!(state.retained, (p*q) % (1 << columns) == n % (1 << columns),
                            "N={n},p={p},q={q},columns={columns}");
                        assert_eq!(state.basis.cells, before);
                        assert!(!state.basis.negative);
                    }
                }
            }
        }
    }
    #[test]
    fn factor_phase_matches_independent_product_and_clears_every_ancilla() {
        for n in [4, 6, 7, 9, 15, 21] {
            let tape = numeral(n);
            let oracle = FactorPhaseOracle::from_n(&tape).unwrap();
            let l = oracle.layout();
            for p in 0..1usize << l.p.len() {
                for q in 0..1usize << l.q.len() {
                    let mut basis = Basis {
                        cells: vec![EVALT; l.cells],
                        negative: false,
                        gates: 0,
                    };
                    for (range, value) in [(l.p.clone(), p), (l.q.clone(), q)] {
                        for (i, cell) in range.enumerate() {
                            basis.cells[cell] = if value >> i & 1 == 1 { EVALF } else { EVALT };
                        }
                    }
                    let before = basis.cells.clone();
                    oracle.lower(&mut basis).unwrap();
                    assert_eq!(
                        basis.negative,
                        p > 1 && q > 1 && p * q == n,
                        "N={n},p={p},q={q}"
                    );
                    assert_eq!(basis.cells, before, "uncleared work: N={n},p={p},q={q}");
                    oracle.lower(&mut basis).unwrap();
                    assert!(!basis.negative);
                    assert_eq!(basis.cells, before);
                }
            }
        }
    }
    #[test]
    fn factor_phase_interferes_with_diffusion_on_a_small_register() {
        // Independent small-state test instrument. Production preparation and
        // lowering retain no amplitudes or candidate table.
        let n = numeral(15);
        let oracle = FactorPhaseOracle::from_n(&n).unwrap();
        let l = oracle.layout();
        let dimension = 1usize << (l.p.len() + l.q.len());
        let mut amplitudes = vec![1.0 / (dimension as f64).sqrt(); dimension];
        let mut signs = Vec::new();
        for label in 0..dimension {
            let mut basis = Basis {
                cells: vec![EVALT; l.cells],
                negative: false,
                gates: 0,
            };
            for (bit, cell) in l.p.clone().chain(l.q.clone()).enumerate() {
                basis.cells[cell] = if label >> bit & 1 == 1 { EVALF } else { EVALT };
            }
            oracle.lower(&mut basis).unwrap();
            signs.push(if basis.negative { -1.0 } else { 1.0 });
        }
        // This finite test chooses its round count. No round limit is installed
        // in the membrane or in the oracle.
        for _ in 0..8 {
            for (amplitude, sign) in amplitudes.iter_mut().zip(&signs) {
                *amplitude *= sign;
            }
            let mean = amplitudes.iter().sum::<f64>() / dimension as f64;
            for amplitude in &mut amplitudes {
                *amplitude = 2.0 * mean - *amplitude;
            }
        }
        let marked_probability: f64 = amplitudes
            .iter()
            .zip(&signs)
            .filter(|(_, sign)| **sign < 0.0)
            .map(|(a, _)| a * a)
            .sum();
        let norm: f64 = amplitudes.iter().map(|a| a * a).sum();
        assert!((norm - 1.0).abs() < 1e-12);
        assert!(
            marked_probability > 0.99,
            "factor-pair probability {marked_probability}"
        );
        let readout = amplitudes
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.abs().partial_cmp(&b.abs()).unwrap())
            .unwrap()
            .0;
        let mask = (1usize << l.p.len()) - 1;
        let (p, q) = (readout & mask, readout >> l.p.len());
        assert!((p == 3 && q == 5) || (p == 5 && q == 3));
    }

    #[test]
    fn preparation_grows_with_source_without_expanding_gate_stream() {
        for width in [32, 64, 256, 2048, 4096] {
            let mut n = vec![EVALT; width];
            n[0] = EVALF;
            n[width - 1] = EVALF;
            let oracle = FactorPhaseOracle::from_n(&n).unwrap();
            assert_eq!(oracle.n(), n);
            assert_eq!(oracle.layout().cells, 6 * width + 3);
            assert_eq!(oracle.layout().p.len(), width);
            assert_eq!(oracle.layout().product.len(), 2 * width);
        }
    }
    #[test]
    fn invalid_sources_fail_before_emitting_gates() {
        for n in [vec![], vec!['H'], vec![EVALT], vec![EVALF]] {
            assert!(FactorPhaseOracle::from_n(&n).is_err());
        }
    }
}
