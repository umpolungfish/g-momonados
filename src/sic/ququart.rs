//! Sixteen SIC weights on P({T,F,t,f}), with the three carrier orders.
//! WH indices use p*4+q. Carrier bits follow the machine: t,f,T,F.
use super::certificate::negative_eigenvalue_residual;
use super::wh::WhSic;
use super::{error, Complex, Operator, Sic, SicCoordinates, SicError, SicFrame, TOLERANCE};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SixteenOutcome(u8);
impl SixteenOutcome {
    pub fn new(mask: u8) -> Result<Self, SicError> {
        if mask < 16 {
            Ok(Self(mask))
        } else {
            Err(error("SIXTEEN_3 mask exceeds four atoms"))
        }
    }
    pub fn mask(self) -> u8 {
        self.0
    }
    pub fn wh_index(self) -> (usize, usize) {
        (self.0 as usize / 4, self.0 as usize % 4)
    }
    pub fn information_le(self, rhs: Self) -> bool {
        self.0 & !rhs.0 == 0
    }
    pub fn truth_le(self, rhs: Self) -> bool {
        (self.0 & 5) & !(rhs.0 & 5) == 0 && (rhs.0 & 10) & !(self.0 & 10) == 0
    }
    pub fn constructivity_le(self, rhs: Self) -> bool {
        (self.0 & 12) & !(rhs.0 & 12) == 0 && (rhs.0 & 3) & !(self.0 & 3) == 0
    }
}

#[derive(Clone, Debug)]
pub struct QuquartState {
    density: Operator,
}
impl QuquartState {
    pub fn new(density: Operator) -> Result<Self, SicError> {
        if density.dimension() != 4 || density.values().iter().any(|z| !z.finite()) {
            return Err(error("ququart density must be a finite 4 by 4 operator"));
        }
        if (density.trace() - Complex::new(1.0, 0.0)).abs2().sqrt() > TOLERANCE {
            return Err(error("ququart density trace must be one"));
        }
        if negative_eigenvalue_residual(&density) > TOLERANCE {
            return Err(error("ququart density must be Hermitian and positive"));
        }
        Ok(Self { density })
    }
    /// Basis order T,F,t,f. The informational arm has rank two.
    pub fn pure(ray: [Complex; 4]) -> Result<Self, SicError> {
        Self::new(Operator::projector(&ray)?)
    }
    pub fn maximally_mixed() -> Self {
        Self::new(Operator::identity(4).scale(0.25)).unwrap()
    }
    pub fn operator(&self) -> &Operator {
        &self.density
    }
    pub fn measure_arms(&self) -> [f64; 3] {
        [
            self.density.get(0, 0).re,
            self.density.get(1, 1).re,
            self.density.get(2, 2).re + self.density.get(3, 3).re,
        ]
    }
    pub fn conditional_arm(&self, arm: usize) -> Result<Self, SicError> {
        let weight = *self
            .measure_arms()
            .get(arm)
            .ok_or_else(|| error("invalid evaluator arm"))?;
        if weight <= 0.0 {
            return Err(error("cannot condition on a zero-weight arm"));
        }
        let selected = |i| match arm {
            0 => i == 0,
            1 => i == 1,
            _ => i >= 2,
        };
        let mut density = Operator::zero(4);
        for i in 0..4 {
            for j in 0..4 {
                if selected(i) && selected(j) {
                    density.set(i, j, self.density.get(i, j).scale(1.0 / weight));
                }
            }
        }
        Self::new(density)
    }
}

pub struct QuquartSic {
    frame: SicFrame,
}
impl Default for QuquartSic {
    fn default() -> Self {
        Self::canonical()
    }
}
impl QuquartSic {
    /// Appleby, quant-ph/0412001, equation 149, cyclic WH(4).
    pub fn canonical() -> Self {
        let a = ((5.0 - 5.0f64.sqrt()) / 40.0).sqrt();
        let c = (2.0 + 2.0f64.sqrt()).sqrt() / 2.0;
        let s = (2.0 - 2.0f64.sqrt()).sqrt() / 2.0;
        let u = (2.0 + 5.0f64.sqrt()).sqrt();
        let wh = WhSic::new(vec![
            Complex::new(2.0 * a * c, 0.0),
            Complex::new(a * (1.0 - u) * s, a * (1.0 + u) * c),
            Complex::new(0.0, 2.0 * a * s),
            Complex::new(a * (1.0 + u) * s, a * (1.0 - u) * c),
        ])
        .unwrap();
        Self {
            frame: SicFrame::new((0..16).map(|i| wh.projector(i).unwrap()).collect()).unwrap(),
        }
    }
    pub fn frame(&self) -> &SicFrame {
        &self.frame
    }
    pub fn split(&self, state: &QuquartState) -> Result<QuquartSicState, SicError> {
        let x = self.frame.split(state.operator())?;
        self.try_weights(std::array::from_fn(|i| x.values[i].re))
    }
    pub fn fuse(&self, weights: &QuquartSicState) -> Result<QuquartState, SicError> {
        QuquartState::new(
            self.frame.fuse(&SicCoordinates {
                dimension: 4,
                values: weights
                    .weights
                    .iter()
                    .map(|p| Complex::new(*p, 0.0))
                    .collect(),
            })?,
        )
    }
    pub fn try_weights(&self, weights: [f64; 16]) -> Result<QuquartSicState, SicError> {
        if weights.iter().any(|p| !p.is_finite() || *p < -TOLERANCE)
            || (weights.iter().sum::<f64>() - 1.0).abs() > TOLERANCE
        {
            return Err(error("invalid SIXTEEN_3 SIC weights"));
        }
        let state = QuquartSicState { weights };
        self.fuse(&state)?;
        Ok(state)
    }
}

#[derive(Clone, Debug)]
pub struct QuquartSicState {
    weights: [f64; 16],
}
impl QuquartSicState {
    pub fn weights(&self) -> &[f64; 16] {
        &self.weights
    }
    pub fn weight(&self, outcome: SixteenOutcome) -> f64 {
        self.weights[outcome.mask() as usize]
    }
    pub fn urgleichung<const M: usize>(&self, conditional: &[[f64; 16]; M]) -> [f64; M] {
        std::array::from_fn(|j| {
            (0..16)
                .map(|i| (5.0 * self.weights[i] - 0.25) * conditional[j][i])
                .sum()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sic::certificate::SicCertificate;
    #[test]
    fn sixteen_outcome_ququart_closure() {
        let sic = QuquartSic::canonical();
        let c = SicCertificate::measure(sic.frame()).unwrap();
        for r in [
            c.closure,
            c.equiangularity,
            c.completeness,
            c.duality,
            c.positivity,
        ] {
            assert!(r < 1e-12, "{r}");
        }
        let mixed = sic.split(&QuquartState::maximally_mixed()).unwrap();
        assert!(mixed
            .weights()
            .iter()
            .all(|p| (*p - 1.0 / 16.0).abs() < 1e-12));
        let conditional: [[f64; 16]; 4] = std::array::from_fn(|j| {
            std::array::from_fn(|i| sic.frame.projectors()[i].get(j, j).re)
        });
        let mut seed = 981u64;
        for _ in 0..128 {
            let mut next = || {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                (seed >> 11) as f64 / (1u64 << 53) as f64 - 0.5
            };
            let mut ray: [Complex; 4] = std::array::from_fn(|_| Complex::new(next(), next()));
            let norm = ray.iter().map(|z| z.abs2()).sum::<f64>().sqrt();
            for z in &mut ray {
                *z = z.scale(1.0 / norm);
            }
            let state = QuquartState::pure(ray).unwrap();
            let p = sic.split(&state).unwrap();
            assert!(
                sic.fuse(&p)
                    .unwrap()
                    .operator()
                    .distance(state.operator())
                    .unwrap()
                    < 1e-12
            );
            assert!((p.weights().iter().map(|p| p * p).sum::<f64>() - 0.1).abs() < 1e-12);
            for (j, q) in p.urgleichung(&conditional).iter().enumerate() {
                assert!((*q - state.operator().get(j, j).re).abs() < 1e-12);
            }
        }
        let b = 1.0 / 2.0f64.sqrt();
        let held = QuquartState::pure([
            Complex::default(),
            Complex::default(),
            Complex::new(b, 0.0),
            Complex::new(0.0, b),
        ])
        .unwrap();
        assert!(
            held.conditional_arm(2)
                .unwrap()
                .operator()
                .distance(held.operator())
                .unwrap()
                < 1e-12
        );
        for i in 0..16 {
            let a = SixteenOutcome::new(i).unwrap();
            assert_eq!(a.wh_index().0 * 4 + a.wh_index().1, i as usize);
            assert!(a.information_le(a) && a.truth_le(a) && a.constructivity_le(a));
        }
        assert!(sic
            .try_weights(std::array::from_fn(|i| if i == 0 { 1.0 } else { 0.0 }))
            .is_err());
    }
}
