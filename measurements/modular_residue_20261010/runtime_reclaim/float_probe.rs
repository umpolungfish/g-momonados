use std::hint::black_box;
fn main() {
    for k in 0..10 {
        let angle = black_box(core::f64::consts::FRAC_PI_2 * k as f64);
        let (s, c) = (angle.sin(), angle.cos());
        println!("angle_{k}={:016x},{:016x},{:016x}", s.to_bits(), c.to_bits(), s.hypot(c).to_bits());
    }
    for (i, (x, y)) in [(0.0, 0.0), (1.0e-16, 0.0), (1.0e-16, 2.0e-16),
                       (0.0, 1.0), (3.0, 4.0)].into_iter().enumerate() {
        let (x, y): (f64, f64) = black_box((x, y));
        println!("hypot_{i}={:016x},{:016x}", x.hypot(y).to_bits(), 0.0f64.max(x.hypot(y)).to_bits());
    }
}
