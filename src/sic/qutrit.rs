//! Three-level density states and the nine-outcome Hesse SIC.
use super::wh::WhSic;
use super::{error, Complex, Operator, Sic, SicCoordinates, SicError, SicFrame, TOLERANCE};

#[derive(Clone, Debug)]
pub struct QutritState {
    density: Operator,
}
impl QutritState {
    pub fn new(density: Operator) -> Result<Self, SicError> {
        if density.dimension() != 3 {
            return Err(error("qutrit density operator must be 3 by 3"));
        }
        if (density.trace() - Complex::new(1.0, 0.0)).abs2().sqrt() > TOLERANCE {
            return Err(error("qutrit density trace must be one"));
        }
        for i in 0..3 {
            if density.get(i, i).re < -TOLERANCE {
                return Err(error("negative qutrit population"));
            }
            for j in 0..3 {
                if (density.get(i, j) - density.get(j, i).conj()).abs2().sqrt() > TOLERANCE {
                    return Err(error("qutrit density must be Hermitian"));
                }
                if i < j
                    && density.get(i, i).re * density.get(j, j).re - density.get(i, j).abs2()
                        < -TOLERANCE
                {
                    return Err(error("qutrit density has a negative principal minor"));
                }
            }
        }
        let a = |i, j| density.get(i, j);
        let det = a(0, 0) * (a(1, 1) * a(2, 2) - a(1, 2) * a(2, 1))
            - a(0, 1) * (a(1, 0) * a(2, 2) - a(1, 2) * a(2, 0))
            + a(0, 2) * (a(1, 0) * a(2, 1) - a(1, 1) * a(2, 0));
        if det.re < -TOLERANCE {
            return Err(error("qutrit density has a negative determinant"));
        }
        Ok(Self { density })
    }
    pub fn pure(amplitudes: [Complex; 3]) -> Result<Self, SicError> {
        if (amplitudes.iter().map(|z| z.abs2()).sum::<f64>() - 1.0).abs() > TOLERANCE {
            return Err(error("qutrit ray must have unit norm"));
        }
        Self::new(Operator::projector(&amplitudes)?)
    }
    pub fn maximally_mixed() -> Self {
        Self::new(Operator::identity(3).scale(1.0 / 3.0)).unwrap()
    }
    pub fn operator(&self) -> &Operator {
        &self.density
    }
    pub fn measure(&self) -> [f64; 3] {
        std::array::from_fn(|i| self.density.get(i, i).re)
    }
    pub fn dephase(&self, lambda: f64) -> Result<Self, SicError> {
        if !lambda.is_finite() || !(0.0..=1.0).contains(&lambda) {
            return Err(error("dephasing coefficient must be between zero and one"));
        }
        let mut density = self.density.clone();
        for i in 0..3 {
            for j in 0..3 {
                if i != j {
                    density.set(i, j, density.get(i, j).scale(lambda));
                }
            }
        }
        Self::new(density)
    }
}

pub struct HesseSic {
    frame: SicFrame,
}
impl Default for HesseSic {
    fn default() -> Self {
        Self::canonical()
    }
}
impl HesseSic {
    pub fn canonical() -> Self {
        let c = 1.0 / 2.0f64.sqrt();
        let wh = WhSic::new(vec![
            Complex::default(),
            Complex::new(c, 0.0),
            Complex::new(-c, 0.0),
        ])
        .unwrap();
        let frame = SicFrame::new((0..9).map(|i| wh.projector(i).unwrap()).collect()).unwrap();
        Self { frame }
    }
    pub fn frame(&self) -> &SicFrame {
        &self.frame
    }
    pub fn analyze(&self, state: &QutritState) -> Result<QutritSicState, SicError> {
        let coordinates = self.frame.split(state.operator())?;
        let probabilities = std::array::from_fn(|i| coordinates.values[i].re);
        QutritSicState::new(probabilities)
    }
    pub fn reconstruct(&self, state: &QutritSicState) -> Result<QutritState, SicError> {
        let coordinates = SicCoordinates {
            dimension: 3,
            values: state.p.iter().map(|p| Complex::new(*p, 0.0)).collect(),
        };
        QutritState::new(self.frame.fuse(&coordinates)?)
    }
}

#[derive(Clone, Debug)]
pub struct QutritSicState {
    p: [f64; 9],
}
impl QutritSicState {
    pub fn new(p: [f64; 9]) -> Result<Self, SicError> {
        if p.iter().any(|p| !p.is_finite() || *p < -TOLERANCE)
            || (p.iter().sum::<f64>() - 1.0).abs() > TOLERANCE
        {
            return Err(error("invalid nine-outcome SIC distribution"));
        }
        let state = Self { p };
        HesseSic::canonical().reconstruct(&state)?;
        Ok(state)
    }
    pub fn probabilities(&self) -> &[f64; 9] {
        &self.p
    }
    pub fn reconstruct(&self) -> Result<QutritState, SicError> {
        HesseSic::canonical().reconstruct(self)
    }
    pub fn dephase(&self, lambda: f64) -> Result<Self, SicError> {
        HesseSic::canonical().analyze(&self.reconstruct()?.dephase(lambda)?)
    }
    /// Signed dual coefficients 4p_i - 1/3 are kept without clamping.
    pub fn urgleichung<const M: usize>(&self, conditional: &[[f64; 9]; M]) -> [f64; M] {
        std::array::from_fn(|j| {
            (0..9)
                .map(|i| (4.0 * self.p[i] - 1.0 / 3.0) * conditional[j][i])
                .sum()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sic::certificate::SicCertificate;
    #[test]
    fn qutrit_nine_outcome_retraction_and_third_level_readout() {
        let frame = HesseSic::canonical();
        let certificate = SicCertificate::measure(frame.frame()).unwrap();
        for residual in [
            certificate.closure,
            certificate.completeness,
            certificate.equiangularity,
            certificate.duality,
            certificate.positivity,
        ] {
            assert!(residual < 1e-12);
        }
        let mut states = vec![QutritState::maximally_mixed()];
        for basis in 0..3 {
            states.push(
                QutritState::pure(std::array::from_fn(|i| {
                    Complex::new(if i == basis { 1.0 } else { 0.0 }, 0.0)
                }))
                .unwrap(),
            );
        }
        let mut seed = 718u64;
        for _ in 0..128 {
            let mut next = || {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                (seed >> 11) as f64 / (1u64 << 53) as f64 - 0.5
            };
            let mut ray: [Complex; 3] = std::array::from_fn(|_| Complex::new(next(), next()));
            let norm = ray.iter().map(|z| z.abs2()).sum::<f64>().sqrt();
            for z in &mut ray {
                *z = z.scale(1.0 / norm);
            }
            states.push(QutritState::pure(ray).unwrap());
        }
        let conditional: [[f64; 9]; 3] = std::array::from_fn(|j| {
            std::array::from_fn(|i| frame.frame.projectors()[i].get(j, j).re)
        });
        for state in states {
            let p = frame.analyze(&state).unwrap();
            assert!(
                p.reconstruct()
                    .unwrap()
                    .operator()
                    .distance(state.operator())
                    .unwrap()
                    < 1e-12
            );
            let reconstructed = p.urgleichung(&conditional);
            for (a, b) in reconstructed.iter().zip(state.measure()) {
                assert!((a - b).abs() < 1e-12);
            }
            let decohered = p.dephase(0.3).unwrap().reconstruct().unwrap();
            assert!(
                decohered
                    .operator()
                    .distance(state.dephase(0.3).unwrap().operator())
                    .unwrap()
                    < 1e-12
            );
        }
        let third = QutritState::pure([
            Complex::default(),
            Complex::default(),
            Complex::new(1.0, 0.0),
        ])
        .unwrap();
        assert_eq!(third.measure(), [0.0, 0.0, 1.0]);
        let mixed = frame.analyze(&QutritState::maximally_mixed()).unwrap();
        assert!(mixed.p.iter().all(|p| (*p - 1.0 / 9.0).abs() < 1e-12));
        // Purity alone cannot validate a qutrit SIC vector. This distribution
        // is positive and normalized, but its reconstructed density is not.
        let mut invalid = Operator::zero(3);
        for (i, value) in [-0.1, 0.5, 0.6].iter().enumerate() {
            invalid.set(i, i, Complex::new(*value, 0.0));
        }
        let coordinates = frame.frame.split(&invalid).unwrap();
        let p: [f64; 9] = std::array::from_fn(|i| coordinates.values[i].re);
        assert!(p.iter().all(|p| *p >= 0.0));
        assert!(p.iter().map(|p| p * p).sum::<f64>() < 1.0 / 6.0);
        assert!(QutritSicState::new(p).is_err());
    }
}
