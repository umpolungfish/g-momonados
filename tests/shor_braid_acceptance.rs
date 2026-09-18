//! shor_braid_acceptance.rs — spec acceptance: braid -> winding -> factor_close.
//! Uses vox_core (vendor/vox) shor_braid + winding_readout; verifies p*q = N.
use vox_core::morphism_factor::{cmp, dec_of, decimal_to_tape, mul};
use vox_core::shor_braid::{shor_braid, shor_factor_via_braid};
use vox_core::winding_readout::{winding_number, winding_number_tape};

fn check(a: &str, n: &str, r_exp: &str) {
    let av = decimal_to_tape(a).unwrap();
    let nv = decimal_to_tape(n).unwrap();
    let (w, lv) = shor_braid(&av, &nv).unwrap();
    let rval: usize = r_exp.parse().unwrap_or(0);
    assert!(
        w.len() <= 8 + 6 * lv + rval.min(64),
        "word O(log N)+comb: len={} levels={}",
        w.len(),
        lv
    );
    let r = dec_of(&winding_number_tape(&w).unwrap());
    assert_eq!(r, r_exp, "winding readout for a={a} N={n}");
    if r_exp.parse::<u64>().map(|v| v <= 64).unwrap_or(false) {
        assert_eq!(winding_number(&w).unwrap(), r_exp.parse::<i64>().unwrap());
    }
    let (p, q) = shor_factor_via_braid(&av, &nv).unwrap();
    let prod = mul(&p, &q);
    assert!(cmp(&prod, &nv) == core::cmp::Ordering::Equal, "p*q=N for N={n}");
    assert!(cmp(&p, &decimal_to_tape("1").unwrap()) == core::cmp::Ordering::Greater);
    assert!(cmp(&p, &nv) == core::cmp::Ordering::Less);
    println!(
        "OK N={n} a={a} r={r} levels={lv} wordlen={} factors={} x {}",
        w.len(),
        dec_of(&p),
        dec_of(&q)
    );
}

#[test]
fn small_n_orders_and_factors() {
    check("7", "15", "4");
    check("2", "21", "6");
    check("2", "35", "12");
    check("2", "77", "30");
    check("2", "143", "60");
    // measured ord_10403(2)=5100 (spec table value 192 is wrong: 2^192 != 1 mod 10403)
    check("2", "10403", "5100");
}
