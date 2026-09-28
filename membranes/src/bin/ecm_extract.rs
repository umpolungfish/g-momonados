// ECM factor-extracting membrane: simplified direct gcd approach
// Track denominator during point arithmetic; gcd(denom, N) reveals factor when denom not invertible

use membranes::{Big, from_dec, to_dec, mul, sub, add, divmod, from_u32, cmp};
use core::cmp::Ordering::*;

fn one() -> Big { from_u32(1) }
fn is_zero(b: &Big) -> bool { b.is_empty() || b.iter().all(|&v| v == 0) }

fn mulmod(a: &Big, b: &Big, n: &Big) -> Big { divmod(&mul(a, b), n).1 }
fn addmod(a: &Big, b: &Big, n: &Big) -> Big { divmod(&add(a, b), n).1 }
fn submod(a: &Big, b: &Big, n: &Big) -> Big {
    if cmp(a, b) != Less { divmod(&sub(a, b), n).1 }
    else { let diff = sub(n, b); addmod(&diff, a, n) }
}

fn gcd(a: &Big, b: &Big) -> Big {
    let (mut x, mut y) = (a.clone(), b.clone());
    while !is_zero(&y) {
        let r = divmod(&x, &y).1;
        x = y;
        y = r;
    }
    x
}

// Point doubling with explicit denominator tracking
// Returns (x_num, x_den) in projective coordinates
fn point_double_proj(x_num: &Big, x_den: &Big, y_num: &Big, y_den: &Big, a: &Big, n: &Big) -> (Big, Big, Big, Big) {
    // Convert to affine: x = x_num/x_den, y = y_num/y_den
    // For simplicity, work with x, y directly and track denominator separately
    
    let x = x_num.clone();
    let y = y_num.clone();
    
    // s = (3x² + a) / (2y)
    let x2 = mulmod(&x, &x, n);
    let num = addmod(&mul(&x2, &from_u32(3)), a, n);  // 3x² + a
    let den = addmod(&y, &y, n);  // 2y
    
    // x' = s² - 2x where s = num/den
    // x' = num²/den² - 2x = (num² - 2x·den²)/den²
    let den2 = mulmod(&den, &den, n);
    let two_x_den2 = mulmod(&mul(&x, &den2), &from_u32(2), n);
    let num2 = mulmod(&num, &num, n);
    let x_new_num = submod(&num2, &two_x_den2, n);
    let x_new_den = den2.clone();
    
    // y' = s(x - x') - y
    // Simplified: just track x coordinate for ECM
    let y_new_num = den.clone();  // Placeholder
    
    (x_new_num, x_new_den, y_new_num, den2)
}

// ECM: multiply point by smooth number, track gcd of denominators
fn ecm_extract(n: &Big, b1: u32, b2: u32) -> Option<(Big, Big)> {
    let a = from_u32(1);  // Curve: y² = x³ + ax + b with a=1, b=1
    
    // Try multiple starting points
    for seed in 2..=100u32 {
        let mut x = from_u32(seed);
        let mut y = from_u32(seed.wrapping_add(1));
        let mut den = one();  // Track denominator
        
        // Stage 1: multiply by all p^e for p ≤ b1
        for p in 2..=b1 {
            let mut count = 0u32;
            let mut pp = from_u32(p);
            while cmp(&pp, n) == Less {
                count += 1;
                pp = mul(&pp, &from_u32(p));
            }
            
            // Double 'count' times
            for _ in 0..count {
                // s = (3x² + a) / (2y)
                let x2 = mulmod(&x, &x, n);
                let num = addmod(&mul(&x2, &from_u32(3)), &a, n);
                let den_new = addmod(&y, &y, n);  // 2y
                
                // Check gcd of denominator with N
                let g = gcd(&den_new, n);
                if cmp(&g, &one()) == Greater && cmp(&g, n) == Less {
                    let co = divmod(n, &g).0;
                    if cmp(&mul(&g, &co), n) == Equal {
                        return Some((g, co));
                    }
                }
                
                // Update point (simplified - just track x)
                let den2 = mulmod(&den_new, &den_new, n);
                let num2 = mulmod(&num, &num, n);
                let two_x = addmod(&x, &x, n);
                let two_x_den2 = mulmod(&two_x, &den2, n);
                x = submod(&num2, &two_x_den2, n);
                y = den2;  // Simplified y update
                den = mulmod(&den, &den_new, n);
            }
            
            // Check accumulated denominator
            let g = gcd(&den, n);
            if cmp(&g, &one()) == Greater && cmp(&g, n) == Less {
                let co = divmod(n, &g).0;
                if cmp(&mul(&g, &co), n) == Equal {
                    return Some((g, co));
                }
            }
        }
        
        // Stage 2: for p in (b1, b2], compute p*P and check
        for p in (b1+1)..=b2 {
            let mut px = x.clone();
            let mut py = y.clone();
            
            for _ in 0..p {
                let x2 = mulmod(&px, &px, n);
                let num = addmod(&mul(&x2, &from_u32(3)), &a, n);
                let den_new = addmod(&py, &py, n);
                
                let g = gcd(&den_new, n);
                if cmp(&g, &one()) == Greater && cmp(&g, n) == Less {
                    let co = divmod(n, &g).0;
                    if cmp(&mul(&g, &co), n) == Equal {
                        return Some((g, co));
                    }
                }
                
                let den2 = mulmod(&den_new, &den_new, n);
                let num2 = mulmod(&num, &num, n);
                let two_px = addmod(&px, &px, n);
                let two_px_den2 = mulmod(&two_px, &den2, n);
                px = submod(&num2, &two_px_den2, n);
                py = den2;
            }
            
            // Check difference: P - p*P
            let diff = submod(&px, &x, n);
            let g = gcd(&diff, n);
            if cmp(&g, &one()) == Greater && cmp(&g, n) == Less {
                let co = divmod(n, &g).0;
                if cmp(&mul(&g, &co), n) == Equal {
                    return Some((g, co));
                }
            }
        }
    }
    
    None
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let n = from_dec(&args[0]);
    let b1: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(100);
    let b2: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1000);
    
    println!("ecm_extract  N={}  b1={}  b2={}", to_dec(&n), b1, b2);
    
    match ecm_extract(&n, b1, b2) {
        Some((p, q)) => {
            println!("  {} = {} x {}  (ECM factor extraction, verified)", to_dec(&n), to_dec(&p), to_dec(&q));
        }
        None => {
            println!("  no factor extracted with bounds b1={}, b2={}", b1, b2);
        }
    }
}