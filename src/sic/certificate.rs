use super::frame::{Complex, Operator, Sic};
use super::{error, SicError};
use crate::belnap_residual::V;

#[derive(Clone, Debug)]
pub struct SicCertificate {
    pub dimension: usize,
    pub normalization: f64,
    pub equiangularity: f64,
    pub completeness: f64,
    pub closure: f64,
    pub positivity: f64,
    pub symmetry: f64,
    pub projector_purity: f64,
    pub duality: f64,
    pub wh_overlap: Option<f64>,
    pub exact: Option<bool>,
}
impl SicCertificate {
    pub fn measure<S: Sic>(frame: &S) -> Result<Self, SicError> {
        let d = frame.dimension();
        let count = d * d;
        let mut c = Self {
            dimension: d,
            normalization: 0.0,
            equiangularity: 0.0,
            completeness: 0.0,
            closure: 0.0,
            positivity: 0.0,
            symmetry: 0.0,
            projector_purity: 0.0,
            duality: 0.0,
            wh_overlap: None,
            exact: None,
        };
        let mut sum = Operator::zero(d);
        for i in 0..count {
            let p = frame.projector(i)?;
            c.normalization = c
                .normalization
                .max((p.trace() - Complex::new(1.0, 0.0)).abs2().sqrt());
            c.projector_purity = c.projector_purity.max(p.multiply(&p)?.distance(&p)?);
            c.positivity = c.positivity.max(negative_eigenvalue_residual(&p));
            sum.add_scaled(&frame.effect(i)?, Complex::new(1.0, 0.0))?;
            for j in 0..count {
                if i != j {
                    c.equiangularity = c.equiangularity.max(
                        (p.trace_product(&frame.projector(j)?)?
                            - Complex::new(1.0 / (d + 1) as f64, 0.0))
                        .abs2()
                        .sqrt(),
                    );
                }
                c.duality = c.duality.max(
                    (frame.effect(i)?.trace_product(&frame.dual(j)?)?
                        - Complex::new(if i == j { 1.0 } else { 0.0 }, 0.0))
                    .abs2()
                    .sqrt(),
                );
            }
        }
        c.completeness = sum.distance(&Operator::identity(d))?;
        for index in 0..d * d {
            for imaginary in [false, true] {
                let mut x = Operator::zero(d);
                x.set(
                    index / d,
                    index % d,
                    Complex::new(
                        if imaginary { 0.0 } else { 1.0 },
                        if imaginary { 1.0 } else { 0.0 },
                    ),
                );
                let coordinates=frame.split(&x)?;
                c.closure = c.closure.max(frame.fuse(&coordinates)?.distance(&x)?);
                let mut moment=Operator::zero(d);
                for (i,z) in coordinates.values.iter().enumerate() {
                    moment.add_scaled(&frame.projector(i)?,z.scale(d as f64))?;
                }
                let coefficient=d as f64/(d+1) as f64;
                let mut expected=x.scale(coefficient);
                expected.add_scaled(&Operator::identity(d),x.trace().scale(coefficient))?;
                c.symmetry=c.symmetry.max(moment.distance(&expected)?);
            }
        }
        Ok(c)
    }
    pub fn report(&self, policy: EvidencePolicy) -> String {
        format!("SIC dimension        : {}\nSIC outcomes         : {}\nSIC normalization    : {:.6e}\nSIC completeness     : {:.6e}\nSIC equiangularity   : {:.6e}\nSIC split/fuse: || μ_SIC δ_SIC(X) - X || = {:.6e}\nSIC positivity       : {:.6e}\nSIC projector purity : {:.6e}\nSIC duality          : {:.6e}\nSIC symmetry         : {:.6e}\nWH overlap max error : {:?}\nExact certificate    : {:?}\nFOUR assessment      : {:?}\n",self.dimension,self.dimension*self.dimension,self.normalization,self.completeness,self.equiangularity,self.closure,self.positivity,self.projector_purity,self.duality,self.symmetry,self.wh_overlap,self.exact,policy.classify(Some(self.closure)))
    }
}

// Hermitian spectrum via the real symmetric embedding [Re -Im; Im Re].
fn negative_eigenvalue_residual(x: &Operator) -> f64 {
    let d = x.dimension();
    let n = 2 * d;
    let mut a = vec![0.0; n * n];
    let mut hermitian = 0.0f64;
    for i in 0..d {
        for j in 0..d {
            let z = x.get(i, j);
            hermitian = hermitian.max((z - x.get(j, i).conj()).abs2().sqrt());
            a[i * n + j] = z.re;
            a[i * n + j + d] = -z.im;
            a[(i + d) * n + j] = z.im;
            a[(i + d) * n + j + d] = z.re;
        }
    }
    for _ in 0..100 * n * n {
        let mut largest = 0.0;
        let mut p = 0;
        let mut q = 0;
        for i in 0..n {
            for j in i + 1..n {
                if a[i * n + j].abs() > largest {
                    largest = a[i * n + j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if largest < 1e-15 {
            break;
        }
        let theta = 0.5 * (2.0 * a[p * n + q]).atan2(a[q * n + q] - a[p * n + p]);
        let (s, c) = theta.sin_cos();
        let app = a[p * n + p];
        let aqq = a[q * n + q];
        let apq = a[p * n + q];
        for k in 0..n {
            if k != p && k != q {
                let kp = a[k * n + p];
                let kq = a[k * n + q];
                a[k * n + p] = c * kp - s * kq;
                a[p * n + k] = a[k * n + p];
                a[k * n + q] = s * kp + c * kq;
                a[q * n + k] = a[k * n + q];
            }
        }
        a[p * n + p] = c * c * app - 2.0 * s * c * apq + s * s * aqq;
        a[q * n + q] = s * s * app + 2.0 * s * c * apq + c * c * aqq;
        a[p * n + q] = 0.0;
        a[q * n + p] = 0.0;
    }
    (0..n)
        .map(|i| (-a[i * n + i]).max(0.0))
        .fold(hermitian, f64::max)
}

#[derive(Clone, Copy)]
pub struct EvidencePolicy {
    support: f64,
    refute: f64,
}
impl EvidencePolicy {
    pub fn new(support: f64, refute: f64) -> Result<Self, SicError> {
        if !support.is_finite() || !refute.is_finite() || support < 0.0 || refute <= support {
            return Err(error("invalid SIC evidence thresholds"));
        }
        Ok(Self { support, refute })
    }
    pub fn classify(self, residual: Option<f64>) -> V {
        match residual {
            Some(x) if x.is_finite() && x >= 0.0 && x <= self.support => V::T,
            Some(x) if x.is_finite() && x >= self.refute => V::F,
            _ => V::N,
        }
    }
    pub fn accumulate(self, existing: V, residual: Option<f64>) -> V {
        existing.join(self.classify(residual))
    }
}
