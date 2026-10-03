//! Four-valued CnotResidual for the Codex Fibonacci.
//! Belnap lattice: N (no info) <= T, F <= B (both). Contradiction never explodes.
//! Run tests: rustc --test belnap_residual.rs -o t && ./t

use std::fmt;

// ---------- Belnap value ----------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum V {
    N,
    T,
    F,
    B,
}

impl V {
    /// Build from raw evidence bits: (supports, refutes).
    pub fn from_evidence(supports: bool, refutes: bool) -> V {
        match (supports, refutes) {
            (false, false) => V::N,
            (true, false) => V::T,
            (false, true) => V::F,
            (true, true) => V::B,
        }
    }
    fn bits(self) -> (bool, bool) {
        match self {
            V::N => (false, false),
            V::T => (true, false),
            V::F => (false, true),
            V::B => (true, true),
        }
    }
    /// Information join: accumulate evidence. N is the unit, B absorbs.
    pub fn join(self, o: V) -> V {
        let (a, b) = (self.bits(), o.bits());
        V::from_evidence(a.0 || b.0, a.1 || b.1)
    }
    /// Consensus meet: keep only what both sources agree on.
    pub fn meet(self, o: V) -> V {
        let (a, b) = (self.bits(), o.bits());
        V::from_evidence(a.0 && b.0, a.1 && b.1)
    }
    /// Negation swaps support and refutation; N and B are fixed points.
    pub fn neg(self) -> V {
        let (s, r) = self.bits();
        V::from_evidence(r, s)
    }
}

// ---------- Frobenius pair on values ----------

/// delta (FSPLIT, ∈): one verdict becomes two arms.
pub fn fsplit(v: V) -> (V, V) {
    (v, v)
}
/// mu (FFUSE, ∋): two arms rejoin by evidence accumulation.
pub fn ffuse(a: V, b: V) -> V {
    a.join(b)
}

// ---------- Residual ----------

/// Raw numerics from a compiled braid word. None = channel never measured.
#[derive(Clone, Copy, Debug)]
pub struct Measured {
    pub comp_err: Option<f64>, // distance of computational-channel unitary from ideal CNOT
    pub leak_prob: Option<f64>, // normalized leakage amplitude residual, not a Born probability
}

#[derive(Clone, Copy, Debug)]
pub struct Thresholds {
    pub eps: f64,    // at or below: evidence the channel is clean
    pub reject: f64, // at or above: evidence the channel is dirty
}

/// Each channel's claim is "this channel is clean".
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CnotResidual {
    pub comp: V,
    pub leak: V,
}

pub fn classify_band(x: Option<f64>, t: Thresholds) -> V {
    match x {
        None => V::N,
        Some(x) if x <= t.eps => V::T,
        Some(x) if x >= t.reject => V::F,
        Some(_) => V::B,
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tier {
    Terminal,     // comp T, leak in {T, N}: every contradiction discharged or never raised
    Crowley,      // comp T, leak B or F: logically works, topologically contaminated
    Inconsistent, // comp B: the gate itself is both right and wrong
    Failed,       // comp F
    Unmeasured,   // comp N
}

impl CnotResidual {
    pub fn tier(&self) -> Tier {
        match (self.comp, self.leak) {
            (V::T, V::T) | (V::T, V::N) => Tier::Terminal,
            (V::T, _) => Tier::Crowley,
            (V::B, _) => Tier::Inconsistent,
            (V::F, _) => Tier::Failed,
            (V::N, _) => Tier::Unmeasured,
        }
    }

    /// Sequential composition of braid words: evidence accumulates per channel.
    pub fn compose(self, o: CnotResidual) -> CnotResidual {
        CnotResidual {
            comp: self.comp.join(o.comp),
            leak: self.leak.join(o.leak),
        }
    }

    /// Fuse two independently measured arms (e.g. forward and reversed braid).
    pub fn fuse(a: CnotResidual, b: CnotResidual) -> CnotResidual {
        a.compose(b)
    }

    /// Split a residual into two arms. With copy-split this is the diagonal.
    pub fn split(self) -> (CnotResidual, CnotResidual) {
        (self, self)
    }

    /// Frobenius closure on the residual: mu(delta(r)) == r.
    /// NOTE: with the copy-split this holds for every value because join is idempotent.
    /// The informative test is `closure_across_arms` below, where arms are measured separately.
    pub fn frobenius_closed(&self) -> bool {
        let (a, b) = self.split();
        CnotResidual::fuse(a, b) == *self
    }
}

/// Informative closure: measure the same word through two procedures; closure holds
/// iff fusing the arms yields no verdict neither arm already contained.
pub fn closure_across_arms(fwd: CnotResidual, rev: CnotResidual) -> bool {
    let fused = CnotResidual::fuse(fwd, rev);
    fused == fwd && fused == rev
}

impl fmt::Display for CnotResidual {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "(comp: {:?}, leak: {:?}) -> {:?}",
            self.comp,
            self.leak,
            self.tier()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const ALL: [V; 4] = [V::N, V::T, V::F, V::B];
    const TH: Thresholds = Thresholds {
        eps: 1e-6,
        reject: 1e-2,
    };

    fn measured(comp_err: Option<f64>, leak_prob: Option<f64>) -> CnotResidual {
        CnotResidual {
            comp: classify_band(comp_err, TH),
            leak: classify_band(leak_prob, TH),
        }
    }

    #[test]
    fn join_is_idempotent_commutative_associative_with_unit() {
        for &a in &ALL {
            assert_eq!(a.join(a), a);
            assert_eq!(a.join(V::N), a);
            for &b in &ALL {
                assert_eq!(a.join(b), b.join(a));
                for &c in &ALL {
                    assert_eq!(a.join(b).join(c), a.join(b.join(c)));
                }
            }
        }
    }

    #[test]
    fn contradiction_does_not_explode() {
        assert_eq!(V::T.join(V::F), V::B);
        assert_eq!(V::B.join(V::T), V::B); // absorbed, nothing else derived
        assert_ne!(V::B, V::T);
    }

    #[test]
    fn negation_involutive_fixes_n_and_b() {
        for &a in &ALL {
            assert_eq!(a.neg().neg(), a);
        }
        assert_eq!(V::N.neg(), V::N);
        assert_eq!(V::B.neg(), V::B);
    }

    #[test]
    fn crowley_case() {
        let r = measured(Some(1e-8), Some(1e-4));
        assert_eq!(
            r,
            CnotResidual {
                comp: V::T,
                leak: V::B
            }
        );
        assert_eq!(r.tier(), Tier::Crowley);
    }

    #[test]
    fn terminal_case_and_unmeasured_leak() {
        assert_eq!(measured(Some(1e-9), Some(1e-9)).tier(), Tier::Terminal);
        assert_eq!(measured(Some(1e-9), None).tier(), Tier::Terminal); // never raised
    }

    #[test]
    fn composition_accumulates_contamination() {
        let clean = CnotResidual {
            comp: V::T,
            leak: V::T,
        };
        let dirty = CnotResidual {
            comp: V::T,
            leak: V::F,
        };
        let c = clean.compose(dirty);
        assert_eq!(c.leak, V::B); // one clean run does not launder a dirty one
        assert_eq!(c.tier(), Tier::Crowley);
    }

    #[test]
    fn frobenius_closure() {
        for &a in &ALL {
            for &b in &ALL {
                assert!(CnotResidual { comp: a, leak: b }.frobenius_closed());
            }
        }
        let clean = CnotResidual {
            comp: V::T,
            leak: V::T,
        };
        let dirty = CnotResidual {
            comp: V::T,
            leak: V::F,
        };
        assert!(!closure_across_arms(clean, dirty)); // arms disagree: closure fails, informatively
        assert!(closure_across_arms(clean, clean));
    }
}
