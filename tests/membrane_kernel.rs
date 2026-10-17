use g_momonados::membrane_kernel::*;
mod tests {
    use super::*;
    #[test]
    fn all_three_orders_retain_lattice_operations_and_both_poles() {
        for order in [Order::Information, Order::Truth, Order::Falsity] {
            for a in 0..16 {
                for b in 0..16 {
                    let x = Sixteen3::from_mask(a).unwrap();
                    let y = Sixteen3::from_mask(b).unwrap();
                    assert!(x.le(x.join(y, order), order));
                    assert!(x.meet(y, order).le(x, order));
                    assert_eq!(x.meet(x.join(y, order), order), x);
                    assert_eq!(x.join(x.meet(y, order), order), x);
                }
            }
        }
        let both = Sixteen3::from_mask(8).unwrap();
        assert!(both.asserts_true() && both.asserts_false());
        assert!(Sixteen3::from_mask(16).is_err());
        assert_eq!(admit().unwrap().mask(), 6);
    }

    #[test]
    fn bounds_reject_nonmonotonicity_and_distinguish_multiple_fixed_points() {
        let identity = core::array::from_fn(|i| Sixteen3::from_mask(i as u8).unwrap());
        assert_eq!(
            fixed_bounds(&identity).unwrap(),
            (
                Sixteen3::from_mask(0).unwrap(),
                Sixteen3::from_mask(15).unwrap()
            )
        );
        let negated = core::array::from_fn(|i| Sixteen3::from_mask(15 ^ i as u8).unwrap());
        assert!(fixed_bounds(&negated).is_err());
    }
}

#[test]
fn native_operations_match_all_exported_lean_tables() {
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../measurements/static_bakers_20261015/native_kernel/lean_tables.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        let seed = Sixteen3::from_mask(row["seed"].as_u64().unwrap() as u8).unwrap();
        let op = row["op"].as_str().unwrap();
        let order = match &op[op.len() - 1..] {
            "i" => Order::Information,
            "t" => Order::Truth,
            "f" => Order::Falsity,
            _ => unreachable!(),
        };
        for (i, expected) in row["table"].as_array().unwrap().iter().enumerate() {
            let x = Sixteen3::from_mask(i as u8).unwrap();
            let actual = if op.starts_with("join") {
                x.join(seed, order)
            } else {
                x.meet(seed, order)
            };
            assert_eq!(
                actual.mask() as u64,
                expected.as_u64().unwrap(),
                "{op} seed={} input={i}",
                seed.mask()
            );
        }
    }
}

#[test]
fn native_continuation_retains_source_and_alternates_beyond_old_cutoff() {
    let clocks: serde_json::Value = serde_json::from_str(include_str!(
        "../measurements/membrane_reentry_20261010/certified_clocks.json"
    ))
    .unwrap();
    let clock = clocks["clocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["retained"].as_u64() == Some(0) && r["seed"].as_u64() == Some(1))
        .unwrap();
    let mut continuation = ResidentContinuation::from_record(&serde_json::json!({
        "y_combinator_func":"resident_y_combinator", "clock_func":"resident_clock", "clock":clock })).unwrap();
    let source = g_momonados::word_tape::WordTape::from_canonical_word(
        include_str!("../../Vox/measurements/static_bakers_20261015/source.imasm").trim(),
    )
    .unwrap();
    assert!(source.bit_len() >= 200);
    let mut carried = (source.clone(), Vec::new());
    for call in 0..1200 {
        let done = continuation
            .advance(|clock| {
                carried.1.push(clock.tick()?);
                Ok(call == 1199)
            })
            .unwrap();
        assert_eq!(done, call == 1199);
        assert_eq!(carried.0, source);
    }
    assert!(carried.1.windows(2).all(|w| w[0] != w[1]));
    let (contractions, calls, ticks, forward, reverse) = continuation.operations();
    assert!(contractions > calls);
    assert_eq!((calls, ticks, forward, reverse), (1200, 1200, 600, 600));
    assert!(continuation
        .advance(|_| panic!("completed Y reentered work"))
        .unwrap());
    assert_eq!(
        continuation.operations(),
        (contractions, calls, ticks, forward, reverse)
    );
    let mut corrupt = continuation.record().clone();
    corrupt["clock"]["table"][0] = 1.into();
    assert!(ResidentContinuation::from_record(&corrupt).is_err());
}

#[test]
fn exact_product_retains_b_evidence_and_rejects_wrong_source_and_unit_arms() {
    use g_momonados::word_tape::WordTape;
    let r: serde_json::Value = serde_json::from_str(include_str!(
        "../measurements/static_bakers_20261015/native_kernel/product_reference.json"
    ))
    .unwrap();
    let word = |key: &str| {
        let n: num_bigint::BigUint = r[key].as_str().unwrap().parse().unwrap();
        let text = g_momonados::native_numeral::encode(&n.to_string());
        WordTape::from_canonical_word(&text).unwrap()
    };
    let source = word("source");
    let left = word("left");
    let right = word("right");
    assert!(source.bit_len() >= 200);
    let e = product_evidence(&source, &left, &right).unwrap();
    assert_eq!(e.mask(), 8);
    assert!(e.asserts_true() && e.asserts_false());
    let wrong = source.add(&WordTape::one());
    assert_eq!(product_evidence(&wrong, &left, &right).unwrap().mask(), 4);
    assert_eq!(
        product_evidence(&source, &WordTape::one(), &source)
            .unwrap()
            .mask(),
        4
    );
}
