use super::certificate::{EvidencePolicy, SicCertificate};
use super::wh::{fft_positive, WhSic};
use super::*;
use crate::belnap_residual::V;
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 2e-12, "{a} != {b}");
}

#[test]
fn tetra_geometry_frame_duality_and_exact_certificate() {
    let tetra = TetraSic::new();
    let mut sum = BlochVector::new(0.0, 0.0, 0.0);
    for i in 0..4 {
        sum = sum.plus(tetra.vertices[i]);
        close(tetra.vertices[i].norm(), 1.0);
        for j in 0..4 {
            close(
                tetra.vertices[i].dot(tetra.vertices[j]),
                if i == j { 1.0 } else { -1.0 / 3.0 },
            );
        }
    }
    close(sum.norm(), 0.0);
    for a in 0..3 {
        for b in 0..3 {
            let value: f64 = tetra
                .vertices
                .iter()
                .map(|r| {
                    let x = [r.x, r.y, r.z];
                    x[a] * x[b]
                })
                .sum();
            close(value, if a == b { 4.0 / 3.0 } else { 0.0 });
        }
    }
    let certificate = SicCertificate::measure(&tetra.frame).unwrap();
    for x in [
        certificate.normalization,
        certificate.completeness,
        certificate.equiangularity,
        certificate.duality,
        certificate.closure,
        certificate.positivity,
        certificate.projector_purity,
    ] {
        assert!(x < 2e-12, "{x}");
    }
    assert!(exact::certify_tetrahedron());
}

#[test]
fn physical_regions_measurements_retraction_urgleichung_and_dephasing() {
    let tetra = TetraSic::new();
    let mut states = vec![
        BlochVector::new(0.0, 0.0, 0.0),
        BlochVector::new(1.0, 0.0, 0.0),
        BlochVector::new(0.0, -1.0, 0.0),
        BlochVector::new(0.0, 0.0, 1.0),
        BlochVector::new(0.0, 0.0, -1.0),
        BlochVector::new(0.2, -0.3, 0.4),
    ];
    let mut seed = 0x123456789abcdefu64;
    for i in 0..2048 {
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            ((seed >> 11) as f64) / ((1u64 << 53) as f64) * 2.0 - 1.0
        };
        let r = BlochVector::new(next(), next(), next());
        let radius = if i % 2 == 0 { 1.0 } else { 0.4 };
        states.push(r.scale(radius / r.norm()));
    }
    for r in states {
        let state = QubitState::new(r).unwrap();
        let p = sic_measure(&state);
        close(p.probabilities().iter().sum(), 1.0);
        let quantum = QubitSicState::new(p.probabilities()).unwrap();
        let restored = quantum.reconstruct().unwrap().bloch();
        close(restored.plus(r.scale(-1.0)).norm(), 0.0);
        let norm2 = p.probabilities().iter().map(|x| x * x).sum::<f64>();
        assert!(norm2 <= 1.0 / 3.0 + TOLERANCE);
        if (r.norm() - 1.0).abs() < TOLERANCE {
            close(norm2, 1.0 / 3.0);
        }
        let direct = truth_measure(&state);
        let conditional = [
            std::array::from_fn(|i| 0.5 * (1.0 + tetra.vertices[i].z)),
            std::array::from_fn(|i| 0.5 * (1.0 - tetra.vertices[i].z)),
        ];
        let q = urgleichung(&p, &conditional);
        close(q[0], direct.truth);
        close(q[1], direct.falsity);
        let dephased = quantum
            .dephase(0.25)
            .unwrap()
            .reconstruct()
            .unwrap()
            .bloch();
        close(dephased.x, r.x * 0.25);
        close(dephased.y, r.y * 0.25);
        close(dephased.z, r.z);
        let density = state.operator();
        let operator_coordinates = tetra.frame.split(&density).unwrap();
        for i in 0..4 {
            close(operator_coordinates.values[i].re, p.probabilities()[i]);
            close(operator_coordinates.values[i].im, 0.0);
        }
        assert!(
            tetra
                .frame
                .fuse(&operator_coordinates)
                .unwrap()
                .distance(&density)
                .unwrap()
                < 2e-12
        );
    }
    assert_eq!(
        sic_measure(&QubitState::new(BlochVector::new(0.0, 0.0, 0.0)).unwrap()).probabilities(),
        [0.25; 4]
    );
    for p in [
        [1.0, 0.0, 0.0, 0.0],
        [f64::NAN, 0.0, 0.0, 1.0],
        [-0.1, 0.1, 0.5, 0.5],
        [0.2; 4],
    ] {
        assert!(QubitSicState::new(p).is_err());
    }
    let policy = EvidencePolicy::new(1e-12, 1e-8).unwrap();
    assert_eq!(policy.classify(Some(1e-10)), V::N);
    assert_eq!(policy.classify(None), V::N);
    assert_eq!(policy.accumulate(V::T, Some(1.0)), V::B);
}

#[test]
fn wh_fft_matches_lazy_displacements_and_general_operator_retraction() {
    let fiducials = [
        vec![
            Complex::new(((1.0 + 1.0 / 3.0f64.sqrt()) / 2.0).sqrt(), 0.0),
            Complex::phase(std::f64::consts::FRAC_PI_4)
                .scale(((1.0 - 1.0 / 3.0f64.sqrt()) / 2.0).sqrt()),
        ],
        vec![
            Complex::default(),
            Complex::new(1.0 / 2.0f64.sqrt(), 0.0),
            Complex::new(-1.0 / 2.0f64.sqrt(), 0.0),
        ],
    ];
    for fiducial in fiducials {
        let wh = WhSic::new(fiducial).unwrap();
        let d = wh.dimension();
        let overlaps = wh.overlaps().unwrap();
        for p in 0..d {
            for q in 0..d {
                let ray = wh.displaced(p, q).unwrap();
                let direct = wh
                    .fiducial()
                    .iter()
                    .zip(ray)
                    .fold(Complex::default(), |s, (a, b)| s + a.conj() * b);
                assert!((direct - overlaps.values[p * d + q]).abs2().sqrt() < 2e-12);
                close(
                    direct.abs2(),
                    if p == 0 && q == 0 {
                        1.0
                    } else {
                        1.0 / (d + 1) as f64
                    },
                );
            }
        }
        let certificate = SicCertificate::measure(&wh).unwrap();
        assert!(certificate.closure < 2e-12);
        assert!(certificate.duality < 2e-12);
        assert!(certificate.equiangularity < 2e-12);
        assert!(certificate.completeness < 2e-12);
        assert!(certificate.projector_purity < 2e-12);
        assert!(certificate.positivity < 2e-12);
    }
    for n in [2, 3, 7, 8, 15, 16] {
        let input: Vec<_> = (0..n)
            .map(|k| Complex::new(k as f64 / n as f64, (k as f64).sin()))
            .collect();
        let fft = fft_positive(&input).unwrap();
        for (q, actual) in fft.iter().enumerate() {
            let direct = input
                .iter()
                .enumerate()
                .fold(Complex::default(), |s, (k, z)| {
                    s + *z * Complex::phase(2.0 * std::f64::consts::PI * (k * q) as f64 / n as f64)
                });
            assert!((*actual - direct).abs2().sqrt() < 1e-11);
        }
    }
}

#[test]
fn fixed_fft_is_reproducible_and_matches_float_wh() {
    use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
    use num_traits::ToPrimitive;
    let format = FixedPointFormat::for_modulus_bits(128).unwrap();
    let scale = format.scale().to_f64().unwrap();
    for d in [2, 3, 7, 8] {
        let ray: Vec<_> = (0..d)
            .map(|k| Complex::new((k as f64 + 1.0) / d as f64, 0.1))
            .collect();
        let fixed: Vec<_> = ray
            .iter()
            .map(|z| FixedComplex::from_f64(z.re, z.im, &format).unwrap())
            .collect();
        let a = fixed::wh_overlaps(&fixed, &format).unwrap();
        let b = fixed::wh_overlaps(&fixed, &format).unwrap();
        let reference = WhSic::new(ray).unwrap().overlaps().unwrap();
        for ((a, b), expected) in a.iter().zip(&b).zip(reference.values) {
            assert_eq!(a.re, b.re);
            assert_eq!(a.im, b.im);
            close(a.re.to_f64().unwrap() / scale, expected.re);
            close(a.im.to_f64().unwrap() / scale, expected.im);
        }
    }
}

#[test]
#[ignore = "requires an available CUDA device and NVRTC"]
fn gpu_overlap_matches_cpu_fft_at_radix_and_bluestein_lengths() {
    for d in [2, 3, 7, 8, 15, 16] {
        let ray = (0..d)
            .map(|k| Complex::new((k as f64 + 1.0) / d as f64, 0.1))
            .collect();
        let wh = WhSic::new(ray).unwrap();
        let cpu = wh.overlaps().unwrap();
        let gpu = gpu::wh_overlaps(&wh).unwrap();
        for (a, b) in cpu.values.iter().zip(&gpu.values) {
            assert!((*a - *b).abs2().sqrt() < 1e-10);
        }
    }
}
