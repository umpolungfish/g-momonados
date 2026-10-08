//! Montgomery ECM over canonical IMASM numeral words.
//!
//! The input, coordinates, curve coefficient, scalar ladder, residues and
//! returned factors all remain `WordTape` values.  The only machine integers
//! are the user-selected bounds and curve index.

use core::cmp::Ordering;
use crate::{add, cmp, divmod, from_u32, is_zero, mul, sub, Big};

#[derive(Clone)]
struct Point { x: Big, z: Big }

enum CurveStart {
    Ready(Point, Big),
    Factor(Big, Big),
    Retry,
}

fn one() -> Big { from_u32(1) }
fn two() -> Big { from_u32(2) }
fn zero() -> Big { from_u32(0) }

fn rem(a: Big, n: &Big) -> Big { divmod(&a, n).1 }
fn addm(a: &Big, b: &Big, n: &Big) -> Big { rem(add(a, b), n) }
fn subm(a: &Big, b: &Big, n: &Big) -> Big {
    if cmp(a, b) != Ordering::Less { rem(sub(a, b), n) }
    else { rem(sub(n, &sub(b, a)), n) }
}
fn mulm(a: &Big, b: &Big, n: &Big) -> Big { rem(mul(a, b), n) }

fn gcd(a: &Big, b: &Big) -> Big {
    let (mut x, mut y) = (a.clone(), b.clone());
    while !is_zero(&y) {
        let r = divmod(&x, &y).1;
        x = y;
        y = r;
    }
    x
}

fn proper_factor(g: Big, n: &Big) -> Option<(Big, Big)> {
    if cmp(&g, &one()) != Ordering::Greater || cmp(&g, n) != Ordering::Less { return None; }
    let (q, r) = divmod(n, &g);
    if is_zero(&r) && mul(&g, &q) == *n { Some((g, q)) } else { None }
}

/// Inverse by Euclid with both Bézout residues carried modulo N.
fn inverse_mod(a: &Big, n: &Big) -> Option<Big> {
    let (mut r, mut next_r) = (n.clone(), rem(a.clone(), n));
    let (mut t, mut next_t) = (zero(), one());
    while !is_zero(&next_r) {
        let (q, new_r) = divmod(&r, &next_r);
        let product = mulm(&q, &next_t, n);
        let new_t = subm(&t, &product, n);
        r = next_r;
        next_r = new_r;
        t = next_t;
        next_t = new_t;
    }
    if r == one() { Some(t) } else { None }
}

/// Suyama's Montgomery curve and its projective point.
fn start_curve(n: &Big, sigma: u32) -> CurveStart {
    let s = from_u32(sigma);
    let u = subm(&mulm(&s, &s, n), &from_u32(5), n);
    let v = mulm(&from_u32(4), &s, n);
    let u2 = mulm(&u, &u, n);
    let u3 = mulm(&u2, &u, n);
    let v3 = mulm(&mulm(&v, &v, n), &v, n);
    let vm_u = subm(&v, &u, n);
    let vm_u3 = mulm(&mulm(&vm_u, &vm_u, n), &vm_u, n);
    let three_u_plus_v = addm(&mulm(&from_u32(3), &u, n), &v, n);
    let numerator = mulm(&vm_u3, &three_u_plus_v, n);
    let denominator = mulm(&mulm(&from_u32(4), &u3, n), &v, n);
    let g = gcd(&denominator, n);
    let denominator_is_unit = g == one();
    if let Some((p, q)) = proper_factor(g, n) { return CurveStart::Factor(p, q); }
    if !denominator_is_unit { return CurveStart::Retry; }
    let Some(inv) = inverse_mod(&denominator, n) else { return CurveStart::Retry; };
    let a = mulm(&numerator, &inv, n);
    let Some(inv_four) = inverse_mod(&from_u32(4), n) else { return CurveStart::Retry; };
    let a24 = mulm(&addm(&a, &two(), n), &inv_four, n);
    CurveStart::Ready(Point { x: u3, z: v3 }, a24)
}

fn double(p: &Point, a24: &Big, n: &Big) -> Point {
    let plus = addm(&p.x, &p.z, n);
    let minus = subm(&p.x, &p.z, n);
    let aa = mulm(&plus, &plus, n);
    let bb = mulm(&minus, &minus, n);
    let c = subm(&aa, &bb, n);
    let x = mulm(&aa, &bb, n);
    let z = mulm(&c, &addm(&bb, &mulm(a24, &c, n), n), n);
    Point { x, z }
}

/// Differential addition, where `difference` is P-Q.
fn add_diff(p: &Point, q: &Point, difference: &Point, n: &Big) -> Point {
    let da = mulm(&subm(&p.x, &p.z, n), &addm(&q.x, &q.z, n), n);
    let cb = mulm(&addm(&p.x, &p.z, n), &subm(&q.x, &q.z, n), n);
    let sum = addm(&da, &cb, n);
    let dif = subm(&da, &cb, n);
    Point {
        x: mulm(&difference.z, &mulm(&sum, &sum, n), n),
        z: mulm(&difference.x, &mulm(&dif, &dif, n), n),
    }
}

/// Montgomery ladder for a machine-sized scalar; all point arithmetic stays
/// on the IMASM words.
fn scalar_mul(p: &Point, scalar: u32, a24: &Big, n: &Big) -> Point {
    let infinity = Point { x: one(), z: zero() };
    let (mut r0, mut r1) = (infinity, p.clone());
    let top = 31 - scalar.leading_zeros();
    for bit in (0..=top).rev() {
        if (scalar >> bit) & 1 == 0 {
            r1 = add_diff(&r0, &r1, p, n);
            r0 = double(&r0, a24, n);
        } else {
            r0 = add_diff(&r0, &r1, p, n);
            r1 = double(&r1, a24, n);
        }
    }
    r0
}

fn is_prime_small(v: u32) -> bool {
    if v < 2 { return false; }
    let mut d = 2u32;
    while d <= v / d {
        if v % d == 0 { return false; }
        d += 1;
    }
    true
}

fn largest_prime_power(p: u32, bound: u32) -> u32 {
    let mut power = p;
    while power <= bound / p { power *= p; }
    power
}

pub fn factor(n: &Big, b1: u32, b2: u32, curves: u32) -> Option<(Big, Big)> {
    if !n.is_odd() { return proper_factor(two(), n); }
    for sigma in 6..6u32.saturating_add(curves) {
        let (mut point, a24) = match start_curve(n, sigma) {
            CurveStart::Factor(p, q) => return Some((p, q)),
            CurveStart::Retry => continue,
            CurveStart::Ready(point, a24) => (point, a24),
        };
        for p in 2..=b1 {
            if !is_prime_small(p) { continue; }
            point = scalar_mul(&point, largest_prime_power(p, b1), &a24, n);
            if let Some(pair) = proper_factor(gcd(&point.z, n), n) { return Some(pair); }
        }
        if b2 > b1 {
            for p in b1.saturating_add(1)..=b2 {
                if !is_prime_small(p) { continue; }
                let stage_two = scalar_mul(&point, p, &a24, n);
                if let Some(pair) = proper_factor(gcd(&stage_two.z, n), n) { return Some(pair); }
            }
        }
    }
    None
}
