//! Shared WordTape squaring-cycle route for the order-labelled membranes.

use crate::{add, cmp, divmod, from_u32, from_word, is_zero, mul, sub, to_word, Big};
use core::cmp::Ordering;

const ORDER_WORD: &str = "⊢⊙⊙∈⊤≻⊥≻≻∋⊡⋈⊣";

fn one() -> Big {
    from_u32(1)
}

fn mulmod(a: &Big, b: &Big, n: &Big) -> Big {
    divmod(&mul(a, b), n).1
}

fn square_mod(value: &Big, n: &Big) -> Big {
    mulmod(value, value, n)
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

fn powmod(a: &Big, exponent: &Big, n: &Big) -> Big {
    let mut result = one();
    let mut base = divmod(a, n).1;
    let mut e = exponent.clone();
    let two = from_u32(2);
    while !is_zero(&e) {
        let (q, bit) = divmod(&e, &two);
        if !is_zero(&bit) {
            result = mulmod(&result, &base, n);
        }
        base = mulmod(&base, &base, n);
        e = q;
    }
    result
}

fn factor_from_order_multiple(a: &Big, order_multiple: &Big, n: &Big) -> Option<(Big, Big)> {
    let two = from_u32(2);
    let n_minus_one = sub(n, &one());
    let mut exponent = order_multiple.clone();
    for _ in 0..4096 {
        let residue = powmod(a, &exponent, n);
        if residue != one() && residue != n_minus_one {
            for candidate in [sub(&residue, &one()), add(&residue, &one())] {
                let factor = gcd(&candidate, n);
                if cmp(&factor, &one()) == Ordering::Greater
                    && cmp(&factor, n) == Ordering::Less
                {
                    let (cofactor, remainder) = divmod(n, &factor);
                    if is_zero(&remainder) && mul(&factor, &cofactor) == *n {
                        return Some((factor, cofactor));
                    }
                }
            }
        }
        let (q, rem) = divmod(&exponent, &two);
        if !is_zero(&rem) || is_zero(&q) {
            break;
        }
        exponent = q;
    }
    None
}

/// Execute one order-cycle frame from its closed IMASM wiring word.
fn advance_order_word(
    n: &Big,
    tortoise: &mut Big,
    hare: &mut Big,
    steps: &mut u64,
    seeded: &mut bool,
) -> bool {
    let mut seed_stage = 0usize;
    let mut tortoise_lane = None;
    let mut hare_lane = None;
    let mut active_hare_lane = None;
    let mut collision = false;

    for mark in ORDER_WORD.chars() {
        match mark {
            '⊢' if !*seeded => {
                *tortoise = from_u32(2);
                *hare = from_u32(2);
            }
            '⊙' if !*seeded => {
                if seed_stage == 0 {
                    *tortoise = square_mod(tortoise, n);
                } else {
                    *hare = square_mod(tortoise, n);
                }
                seed_stage += 1;
            }
            '∈' => {
                *seeded = true;
                tortoise_lane = Some(tortoise.clone());
                hare_lane = Some(hare.clone());
            }
            '⊤' => active_hare_lane = Some(false),
            '⊥' => active_hare_lane = Some(true),
            '≻' => match active_hare_lane {
                Some(false) => {
                    let lane = tortoise_lane.as_mut().expect("order T lane is open");
                    *lane = square_mod(lane, n);
                }
                Some(true) => {
                    let lane = hare_lane.as_mut().expect("order F lane is open");
                    *lane = square_mod(lane, n);
                }
                None => {}
            },
            '∋' => {
                *tortoise = tortoise_lane.take().expect("order T lane reaches its fuse");
                *hare = hare_lane.take().expect("order F lane reaches its fuse");
                active_hare_lane = None;
            }
            '⊡' => *steps += 1,
            '⋈' => collision = tortoise == hare,
            '⊣' => return collision,
            _ => {}
        }
    }
    collision
}

/// Run the shared classical squaring-cycle route on a canonical numeral word.
pub fn run(name: &str, args: &[String], default_cap: u64) {
    if args.is_empty() || args.len() > 2 {
        eprintln!("usage: {name} <canonical-IMASM-N> [squaring-cap]");
        std::process::exit(2);
    }
    let n = from_word(&args[0]).unwrap_or_else(|| {
        eprintln!("expected canonical IMASM numeral word");
        std::process::exit(2)
    });
    let cap = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(default_cap);
    let base = from_u32(2);
    let mut tortoise = base.clone();
    let mut hare = base.clone();
    let mut steps = 1u64;
    let mut met = false;
    let mut seeded = false;
    while steps < cap {
        if advance_order_word(&n, &mut tortoise, &mut hare, &mut steps, &mut seeded) {
            met = true;
            break;
        }
    }

    println!("{name} input bits={}", n.bit_len());
    if !met {
        println!("  no squaring-cycle collision within {cap} steps");
        return;
    }

    let two = from_u32(2);
    let mut power_of_two = one();
    for _ in 0..steps {
        power_of_two = mul(&power_of_two, &two);
    }
    let order_multiple = mul(&power_of_two, &sub(&power_of_two, &one()));
    match factor_from_order_multiple(&base, &order_multiple, &n) {
        Some((p, q)) => println!(
            "  factor_word={} cofactor_word={} product_closes=true",
            to_word(&p),
            to_word(&q)
        ),
        None => println!(
            "  R=2^{steps}·(2^{steps}-1) produced no nontrivial gcd"
        ),
    }
}
