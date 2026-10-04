use super::{error, SicError};
use std::ops::{Add, Mul, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}
impl Complex {
    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }
    pub fn conj(self) -> Self {
        Self::new(self.re, -self.im)
    }
    pub fn abs2(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
    pub fn scale(self, x: f64) -> Self {
        Self::new(self.re * x, self.im * x)
    }
    pub fn phase(angle: f64) -> Self {
        Self::new(angle.cos(), angle.sin())
    }
    pub fn finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }
}
impl Add for Complex {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self::new(self.re + b.re, self.im + b.im)
    }
}
impl Sub for Complex {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self::new(self.re - b.re, self.im - b.im)
    }
}
impl Mul for Complex {
    type Output = Self;
    fn mul(self, b: Self) -> Self {
        Self::new(
            self.re * b.re - self.im * b.im,
            self.re * b.im + self.im * b.re,
        )
    }
}

#[derive(Clone, Debug)]
pub struct Operator {
    d: usize,
    values: Vec<Complex>,
}
impl Operator {
    pub fn new(d: usize, values: Vec<Complex>) -> Result<Self, SicError> {
        if d == 0 || d.checked_mul(d) != Some(values.len()) || values.iter().any(|x| !x.finite()) {
            return Err(error("invalid operator shape or scalar"));
        }
        Ok(Self { d, values })
    }
    pub fn zero(d: usize) -> Self {
        Self {
            d,
            values: vec![Complex::default(); d * d],
        }
    }
    pub fn identity(d: usize) -> Self {
        let mut a = Self::zero(d);
        for i in 0..d {
            a.values[i * d + i].re = 1.0;
        }
        a
    }
    pub fn dimension(&self) -> usize {
        self.d
    }
    pub fn values(&self) -> &[Complex] {
        &self.values
    }
    pub fn get(&self, i: usize, j: usize) -> Complex {
        self.values[i * self.d + j]
    }
    pub fn set(&mut self, i: usize, j: usize, z: Complex) {
        self.values[i * self.d + j] = z;
    }
    pub fn projector(ray: &[Complex]) -> Result<Self, SicError> {
        if ray.is_empty() || ray.iter().any(|z| !z.finite()) {
            return Err(error("invalid SIC ray"));
        }
        let d = ray.len();
        Self::new(
            d,
            (0..d * d).map(|k| ray[k / d] * ray[k % d].conj()).collect(),
        )
    }
    pub fn scale(&self, s: f64) -> Self {
        Self {
            d: self.d,
            values: self.values.iter().map(|z| z.scale(s)).collect(),
        }
    }
    pub fn add_scaled(&mut self, b: &Self, z: Complex) -> Result<(), SicError> {
        if self.d != b.d || !z.finite() {
            return Err(error(
                "operator dimensions differ or coefficient is invalid",
            ));
        }
        for (a, b) in self.values.iter_mut().zip(&b.values) {
            *a = *a + *b * z;
        }
        Ok(())
    }
    pub fn trace(&self) -> Complex {
        (0..self.d).fold(Complex::default(), |a, i| a + self.get(i, i))
    }
    pub fn trace_product(&self, b: &Self) -> Result<Complex, SicError> {
        if self.d != b.d {
            return Err(error("operator dimensions differ"));
        }
        Ok((0..self.d * self.d).fold(Complex::default(), |a, k| {
            a + self.values[k] * b.get(k % self.d, k / self.d)
        }))
    }
    pub fn multiply(&self, b: &Self) -> Result<Self, SicError> {
        if self.d != b.d {
            return Err(error("operator dimensions differ"));
        }
        Self::new(
            self.d,
            (0..self.d * self.d)
                .map(|k| {
                    (0..self.d).fold(Complex::default(), |z, j| {
                        z + self.get(k / self.d, j) * b.get(j, k % self.d)
                    })
                })
                .collect(),
        )
    }
    pub fn distance(&self, b: &Self) -> Result<f64, SicError> {
        if self.d != b.d {
            return Err(error("operator dimensions differ"));
        }
        Ok(self
            .values
            .iter()
            .zip(&b.values)
            .map(|(a, b)| (*a - *b).abs2())
            .sum::<f64>()
            .sqrt())
    }
}

#[derive(Clone, Debug)]
pub struct SicCoordinates {
    pub dimension: usize,
    pub values: Vec<Complex>,
}
pub trait Sic {
    fn dimension(&self) -> usize;
    fn projector(&self, index: usize) -> Result<Operator, SicError>;
    fn effect(&self, index: usize) -> Result<Operator, SicError> {
        Ok(self.projector(index)?.scale(1.0 / self.dimension() as f64))
    }
    fn dual(&self, index: usize) -> Result<Operator, SicError> {
        let mut a = self.projector(index)?.scale((self.dimension() + 1) as f64);
        a.add_scaled(
            &Operator::identity(self.dimension()),
            Complex::new(-1.0, 0.0),
        )?;
        Ok(a)
    }
    fn split(&self, x: &Operator) -> Result<SicCoordinates, SicError> {
        if x.dimension() != self.dimension() {
            return Err(error("analysis operator has the wrong dimension"));
        }
        let values = (0..self.dimension() * self.dimension())
            .map(|i| x.trace_product(&self.effect(i)?))
            .collect::<Result<_, _>>()?;
        Ok(SicCoordinates {
            dimension: self.dimension(),
            values,
        })
    }
    fn fuse(&self, x: &SicCoordinates) -> Result<Operator, SicError> {
        if x.dimension != self.dimension()
            || x.values.len() != self.dimension() * self.dimension()
            || x.values.iter().any(|z| !z.finite())
        {
            return Err(error("invalid SIC coordinate shape or scalar"));
        }
        let mut out = Operator::zero(self.dimension());
        for (i, z) in x.values.iter().enumerate() {
            out.add_scaled(&self.dual(i)?, *z)?;
        }
        Ok(out)
    }
}

pub struct SicFrame {
    d: usize,
    projectors: Vec<Operator>,
    effects: Vec<Operator>,
    duals: Vec<Operator>,
}
impl SicFrame {
    pub fn projectors(&self) -> &[Operator] {
        &self.projectors
    }
    pub fn effects(&self) -> &[Operator] {
        &self.effects
    }
    pub fn duals(&self) -> &[Operator] {
        &self.duals
    }
    pub fn new(projectors: Vec<Operator>) -> Result<Self, SicError> {
        let d = projectors
            .first()
            .ok_or_else(|| error("empty SIC frame"))?
            .dimension();
        if projectors.len() != d * d || projectors.iter().any(|p| p.dimension() != d) {
            return Err(error("SIC frame must contain d squared projectors"));
        }
        let effects = projectors.iter().map(|p| p.scale(1.0 / d as f64)).collect();
        let duals = projectors
            .iter()
            .map(|p| {
                let mut a = p.scale((d + 1) as f64);
                a.add_scaled(&Operator::identity(d), Complex::new(-1.0, 0.0))
                    .unwrap();
                a
            })
            .collect();
        Ok(Self {
            d,
            projectors,
            effects,
            duals,
        })
    }
}
impl Sic for SicFrame {
    fn dimension(&self) -> usize {
        self.d
    }
    fn projector(&self, i: usize) -> Result<Operator, SicError> {
        self.projectors
            .get(i)
            .cloned()
            .ok_or_else(|| error("SIC vertex index out of range"))
    }
    fn effect(&self, i: usize) -> Result<Operator, SicError> {
        self.effects
            .get(i)
            .cloned()
            .ok_or_else(|| error("SIC vertex index out of range"))
    }
    fn dual(&self, i: usize) -> Result<Operator, SicError> {
        self.duals
            .get(i)
            .cloned()
            .ok_or_else(|| error("SIC vertex index out of range"))
    }
}
