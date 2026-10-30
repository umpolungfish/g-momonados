//! Complex arithmetic copied from G-mOMonadOS/src/fibonacci_qc.rs.
#![allow(dead_code)]
fn sqrt(x: f64) -> f64 { x.sqrt() }
fn cos(x: f64) -> f64 { x.cos() }
fn sin(x: f64) -> f64 { x.sin() }
fn atan2(y: f64, x: f64) -> f64 { y.atan2(x) }
/// A complex number with f64 real and imaginary parts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const fn new(re: f64, im: f64) -> Self { Complex { re, im } }
    pub const fn zero() -> Self { Complex { re: 0.0, im: 0.0 } }
    pub const fn one() -> Self { Complex { re: 1.0, im: 0.0 } }
    pub const fn i() -> Self { Complex { re: 0.0, im: 1.0 } }

    pub fn from_polar(r: f64, theta: f64) -> Self {
        Complex { re: r * cos(theta), im: r * sin(theta) }
    }
    pub fn conj(&self) -> Self { Complex { re: self.re, im: -self.im } }
    pub fn norm_sq(&self) -> f64 { self.re * self.re + self.im * self.im }
    pub fn norm(&self) -> f64 { sqrt(self.norm_sq()) }
    pub fn arg(&self) -> f64 { atan2(self.im, self.re) }
    pub fn scale(&self, s: f64) -> Self { Complex { re: self.re * s, im: self.im * s } }
}

impl core::ops::Add for Complex {
    type Output = Complex;
    fn add(self, rhs: Complex) -> Complex {
        Complex { re: self.re + rhs.re, im: self.im + rhs.im }
    }
}

impl core::ops::Sub for Complex {
    type Output = Complex;
    fn sub(self, rhs: Complex) -> Complex {
        Complex { re: self.re - rhs.re, im: self.im - rhs.im }
    }
}

impl core::ops::Mul for Complex {
    type Output = Complex;
    fn mul(self, rhs: Complex) -> Complex {
        Complex {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }
}

impl core::ops::Mul<f64> for Complex {
    type Output = Complex;
    fn mul(self, rhs: f64) -> Complex { Complex { re: self.re * rhs, im: self.im * rhs } }
}

impl core::ops::Div for Complex {
    type Output = Complex;
    fn div(self, rhs: Complex) -> Complex {
        let denom = rhs.norm_sq();
        Complex {
            re: (self.re * rhs.re + self.im * rhs.im) / denom,
            im: (self.im * rhs.re - self.re * rhs.im) / denom,
        }
    }
}

impl core::ops::Div<f64> for Complex {
    type Output = Complex;
    fn div(self, rhs: f64) -> Complex { Complex { re: self.re / rhs, im: self.im / rhs } }
}

impl core::ops::Neg for Complex {
    type Output = Complex;
    fn neg(self) -> Complex { Complex { re: -self.re, im: -self.im } }
}


