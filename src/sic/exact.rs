//! Exact number field Q(sqrt(3)); numerical embedding is a separate operation.
use super::SicError;
use num_bigint::BigInt;
use num_traits::{One, Signed, ToPrimitive, Zero};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Qsqrt3 {
    pub a: BigInt,
    pub b: BigInt,
    pub denominator: BigInt,
}
impl Qsqrt3 {
    pub fn new(a: BigInt, b: BigInt, denominator: BigInt) -> Result<Self, SicError> {
        if denominator.is_zero() {
            return Err(super::error("zero algebraic denominator"));
        }
        let sign = if denominator.is_negative() {
            -BigInt::one()
        } else {
            BigInt::one()
        };
        let (mut a, mut b, mut d) = (a * &sign, b * &sign, denominator * sign);
        fn gcd(mut a: BigInt, mut b: BigInt) -> BigInt {
            a = a.abs();
            b = b.abs();
            while !b.is_zero() {
                let r = &a % &b;
                a = b;
                b = r;
            }
            a
        }
        let g = gcd(gcd(a.clone(), b.clone()), d.clone());
        a /= &g;
        b /= &g;
        d /= &g;
        Ok(Self {
            a,
            b,
            denominator: d,
        })
    }
    pub fn rational(a: i64, d: i64) -> Self {
        Self::new(a.into(), 0.into(), d.into()).unwrap()
    }
    pub fn plus(&self, b: &Self) -> Self {
        Self::new(
            &self.a * &b.denominator + &b.a * &self.denominator,
            &self.b * &b.denominator + &b.b * &self.denominator,
            &self.denominator * &b.denominator,
        )
        .unwrap()
    }
    pub fn negate(&self) -> Self {
        Self::new(-&self.a, -&self.b, self.denominator.clone()).unwrap()
    }
    pub fn times(&self, b: &Self) -> Self {
        Self::new(
            &self.a * &b.a + &self.b * &b.b * 3u8,
            &self.a * &b.b + &self.b * &b.a,
            &self.denominator * &b.denominator,
        )
        .unwrap()
    }
    /// Positive sqrt(3) embedding. Exact arithmetic never calls this method.
    pub fn embed(&self) -> Result<f64, SicError> {
        Ok((self
            .a
            .to_f64()
            .ok_or_else(|| super::error("algebraic embedding overflow"))?
            + self
                .b
                .to_f64()
                .ok_or_else(|| super::error("algebraic embedding overflow"))?
                * 3.0f64.sqrt())
            / self
                .denominator
                .to_f64()
                .ok_or_else(|| super::error("algebraic embedding overflow"))?)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactComplex {
    pub re: Qsqrt3,
    pub im: Qsqrt3,
}
impl ExactComplex {
    pub fn rational(n: i64, d: i64) -> Self {
        Self {
            re: Qsqrt3::rational(n, d),
            im: Qsqrt3::rational(0, 1),
        }
    }
    pub fn plus(&self, b: &Self) -> Self {
        Self {
            re: self.re.plus(&b.re),
            im: self.im.plus(&b.im),
        }
    }
    pub fn times(&self, b: &Self) -> Self {
        Self {
            re: self.re.times(&b.re).plus(&self.im.times(&b.im).negate()),
            im: self.re.times(&b.im).plus(&self.im.times(&b.re)),
        }
    }
}
type Matrix = [ExactComplex; 4];
fn product(a: &Matrix, b: &Matrix) -> Matrix {
    std::array::from_fn(|k| {
        a[2 * (k / 2)]
            .times(&b[k % 2])
            .plus(&a[2 * (k / 2) + 1].times(&b[2 + k % 2]))
    })
}
fn trace(a: &Matrix) -> ExactComplex {
    a[0].plus(&a[3])
}
fn scale(a: &Matrix, n: i64, d: i64) -> Matrix {
    std::array::from_fn(|k| a[k].times(&ExactComplex::rational(n, d)))
}
pub fn tetra_projectors() -> [Matrix; 4] {
    let signs = [[1, 1, 1], [1, -1, -1], [-1, 1, -1], [-1, -1, 1]];
    std::array::from_fn(|i| {
        let [x, y, z] = signs[i];
        let diag = |s: i64| ExactComplex {
            re: Qsqrt3::new(3.into(), s.into(), 6.into()).unwrap(),
            im: Qsqrt3::rational(0, 1),
        };
        let off = |sy: i64| ExactComplex {
            re: Qsqrt3::new(0.into(), x.into(), 6.into()).unwrap(),
            im: Qsqrt3::new(0.into(), sy.into(), 6.into()).unwrap(),
        };
        [diag(z), off(-y), off(y), diag(-z)]
    })
}
/// Exact equiangularity, completeness, duality and retraction on all matrix units.
pub fn certify_tetrahedron() -> bool {
    let p = tetra_projectors();
    let e = p.each_ref().map(|a| scale(a, 1, 2));
    let d = p.each_ref().map(|a| {
        let mut b = scale(a, 3, 1);
        b[0] = b[0].plus(&ExactComplex::rational(-1, 1));
        b[3] = b[3].plus(&ExactComplex::rational(-1, 1));
        b
    });
    for i in 0..4 {
        if trace(&p[i]) != ExactComplex::rational(1, 1) || product(&p[i], &p[i]) != p[i] {
            return false;
        }
        for j in 0..4 {
            if trace(&product(&p[i], &p[j]))
                != ExactComplex::rational(if i == j { 1 } else { 1 }, if i == j { 1 } else { 3 })
            {
                return false;
            }
            if trace(&product(&e[i], &d[j]))
                != ExactComplex::rational(if i == j { 1 } else { 0 }, 1)
            {
                return false;
            }
        }
    }
    let sum: Matrix = std::array::from_fn(|k| {
        e.iter()
            .fold(ExactComplex::rational(0, 1), |z, a| z.plus(&a[k]))
    });
    if sum
        != [
            ExactComplex::rational(1, 1),
            ExactComplex::rational(0, 1),
            ExactComplex::rational(0, 1),
            ExactComplex::rational(1, 1),
        ]
    {
        return false;
    }
    for basis in 0..4 {
        let x: Matrix =
            std::array::from_fn(|k| ExactComplex::rational(if k == basis { 1 } else { 0 }, 1));
        let coordinates = e.each_ref().map(|a| trace(&product(&x, a)));
        let reconstructed: Matrix = std::array::from_fn(|k| {
            (0..4).fold(ExactComplex::rational(0, 1), |z, i| {
                z.plus(&coordinates[i].times(&d[i][k]))
            })
        });
        if reconstructed != x {
            return false;
        }
    }
    true
}
