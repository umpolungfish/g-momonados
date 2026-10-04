use super::frame::{Complex, Operator, SicFrame};
use super::{error, SicError, TOLERANCE};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlochVector {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
impl BlochVector {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    pub fn dot(self, b: Self) -> f64 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }
    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }
    pub fn scale(self, s: f64) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
    pub fn plus(self, b: Self) -> Self {
        Self::new(self.x + b.x, self.y + b.y, self.z + b.z)
    }
    pub fn finite(self) -> bool {
        [self.x, self.y, self.z].iter().all(|x| x.is_finite())
    }
}
#[derive(Clone, Copy, Debug)]
pub struct QubitState {
    r: BlochVector,
}
impl QubitState {
    pub fn new(r: BlochVector) -> Result<Self, SicError> {
        if !r.finite() || r.norm() > 1.0 + TOLERANCE {
            return Err(error("Bloch vector is outside the qubit ball"));
        }
        Ok(Self { r })
    }
    pub fn ray(theta: f64, phi: f64) -> Result<Self, SicError> {
        Self::new(BlochVector::new(
            theta.sin() * phi.cos(),
            theta.sin() * phi.sin(),
            theta.cos(),
        ))
    }
    pub fn bloch(&self) -> BlochVector {
        self.r
    }
    pub fn operator(&self) -> Operator {
        Operator::new(
            2,
            vec![
                Complex::new((1.0 + self.r.z) / 2.0, 0.0),
                Complex::new(self.r.x / 2.0, -self.r.y / 2.0),
                Complex::new(self.r.x / 2.0, self.r.y / 2.0),
                Complex::new((1.0 - self.r.z) / 2.0, 0.0),
            ],
        )
        .unwrap()
    }
    pub fn dephase(&self, lambda: f64) -> Result<Self, SicError> {
        if !lambda.is_finite() || !(0.0..=1.0).contains(&lambda) {
            return Err(error("dephasing coefficient must be between zero and one"));
        }
        Self::new(BlochVector::new(
            self.r.x * lambda,
            self.r.y * lambda,
            self.r.z,
        ))
    }
}
#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SicVertex {
    N = 0,
    T = 1,
    F = 2,
    B = 3,
}
#[derive(Clone, Copy, Debug)]
pub struct SicDistribution {
    p: [f64; 4],
}
impl SicDistribution {
    pub fn probabilities(&self) -> [f64; 4] {
        self.p
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Simplex4 {
    p: [f64; 4],
}
impl Simplex4 {
    pub fn new(p: [f64; 4]) -> Result<Self, SicError> {
        if p.iter().any(|x| !x.is_finite() || *x < -TOLERANCE)
            || (p.iter().sum::<f64>() - 1.0).abs() > TOLERANCE
        {
            return Err(error("invalid normalized simplex point"));
        }
        Ok(Self { p })
    }
    pub fn probabilities(&self) -> [f64; 4] {
        self.p
    }
    pub fn try_quantum(self) -> Result<QubitSicState, SicError> {
        if self.p.iter().map(|x| x * x).sum::<f64>() > 1.0 / 3.0 + TOLERANCE {
            return Err(error("simplex point is outside the qubit SIC region"));
        }
        let state=QubitSicState {
            distribution: SicDistribution { p: self.p },
        };
        state.reconstruct()?;
        Ok(state)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct QubitSicState {
    distribution: SicDistribution,
}
impl QubitSicState {
    pub fn new(p: [f64; 4]) -> Result<Self, SicError> {
        Simplex4::new(p)?.try_quantum()
    }
    pub fn distribution(&self) -> &SicDistribution {
        &self.distribution
    }
    pub fn reconstruct(&self) -> Result<QubitState, SicError> {
        TetraSic::new().fuse(&self.distribution)
    }
    pub fn dephase(&self, lambda: f64) -> Result<Self, SicError> {
        Ok(TetraSic::new().split(&self.reconstruct()?.dephase(lambda)?))
    }
}
pub struct BinaryDistribution {
    pub truth: f64,
    pub falsity: f64,
}
pub fn truth_measure(state: &QubitState) -> BinaryDistribution {
    BinaryDistribution {
        truth: (1.0 + state.r.z) / 2.0,
        falsity: (1.0 - state.r.z) / 2.0,
    }
}
pub fn sic_measure(state: &QubitState) -> SicDistribution {
    TetraSic::new().split(state).distribution
}

pub fn truth_measure_axis(state:&QubitState,axis:BlochVector)->Result<BinaryDistribution,SicError> {
    if !axis.finite() || (axis.norm()-1.0).abs()>TOLERANCE {return Err(error("Boolean measurement axis must be a unit vector"));}
    let overlap=state.r.dot(axis);
    Ok(BinaryDistribution {truth:(1.0+overlap)/2.0,falsity:(1.0-overlap)/2.0})
}
pub struct TetraSic {
    pub vertices: [BlochVector; 4],
    pub frame: SicFrame,
}
impl Default for TetraSic {
    fn default() -> Self {
        Self::new()
    }
}
impl TetraSic {
    pub fn new() -> Self {
        let c = 1.0 / 3.0f64.sqrt();
        let vertices = [
            BlochVector::new(c, c, c),
            BlochVector::new(c, -c, -c),
            BlochVector::new(-c, c, -c),
            BlochVector::new(-c, -c, c),
        ];
        let projectors = vertices
            .iter()
            .map(|&r| QubitState::new(r).unwrap().operator())
            .collect();
        Self {
            vertices,
            frame: SicFrame::new(projectors).unwrap(),
        }
    }
    pub fn split(&self, state: &QubitState) -> QubitSicState {
        QubitSicState {
            distribution: SicDistribution {
                p: std::array::from_fn(|i| 0.25 * (1.0 + state.r.dot(self.vertices[i]))),
            },
        }
    }
    pub fn fuse(&self, p: &SicDistribution) -> Result<QubitState, SicError> {
        QubitState::new((0..4).fold(BlochVector::new(0.0, 0.0, 0.0), |r, i| {
            r.plus(self.vertices[i].scale(3.0 * p.p[i]))
        }))
    }
}
