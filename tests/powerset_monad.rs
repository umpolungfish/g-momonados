use g_momonados::phase_unbraid::{FixedComplex, FixedPointFormat};
use g_momonados::ququart_decision::DecisionArena;
use g_momonados::reentry_powerset::{multiply, powerset_bind, unit, Held};
use g_momonados::word_tape::WordTape;
use std::collections::BTreeMap;

#[test]
fn finite_powerset_monad_laws_hold_exhaustively_on_three_elements() {
    let subsets: Vec<Held<u8>> = (0u8..8)
        .map(|mask| (0u8..3).filter(|bit| mask & (1 << bit) != 0).collect())
        .collect();
    let arrows: Vec<BTreeMap<u8, Held<u8>>> = (0usize..512)
        .map(|code| {
            (0u8..3)
                .map(|x| {
                    let mask = ((code / 8usize.pow(x as u32)) % 8) as u8;
                    (x, subsets[mask as usize].clone())
                })
                .collect()
        })
        .collect();
    let apply = |set: Held<u8>, arrow: &BTreeMap<u8, Held<u8>>| powerset_bind(set, arrow);
    for x in 0u8..3 {
        for arrow in &arrows {
            assert_eq!(apply(unit(x), arrow), arrow[&x]);
        }
    }
    let identity: BTreeMap<_, _> = (0u8..3).map(|x| (x, unit(x))).collect();
    for set in &subsets {
        assert!(apply(set.clone(), &arrows[0]).is_empty());
        assert_eq!(apply(set.clone(), &identity), *set);
        for f in &arrows {
            let first = apply(set.clone(), f);
            for g in &arrows {
                let left = apply(first.clone(), g);
                let composite: BTreeMap<_, _> =
                    f.iter().map(|(&x, ys)| (x, apply(ys.clone(), g))).collect();
                assert_eq!(left, apply(set.clone(), &composite));
            }
        }
    }
}

#[test]
fn powerset_bind_laws_hold_for_complete_correlated_diagram_roots() {
    let source_word = g_momonados::native_numeral::encode(
        include_str!("../measurements/powerset_resident_20261015/source_n.txt").trim(),
    );
    let source = WordTape::from_canonical_word(&source_word).unwrap();
    assert_eq!(source.bit_len(), 128);
    let n = g_momonados::native_numeral::decode(source.as_word()).unwrap();
    let base = n.clone() >> 1usize;
    assert!(&base + 7u8 < n);
    let mut arena = DecisionArena::new(source.bit_len());
    let format = FixedPointFormat::for_modulus_bits(source.bit_len() as u64).unwrap();
    let real = arena.basis(&base, 2.into());
    let imaginary = arena.basis(&(&base + 3u8), 1.into());
    let imaginary = arena.scale(
        imaginary,
        &FixedComplex {
            re: 0.into(),
            im: format.scale(),
        },
        &format,
    );
    let original = arena.sum(real, imaginary);
    let alternate = arena.flip(original, 2);
    let held: Held<_> = [original, alternate].into_iter().collect();
    // Arena construction is preparation. Bind observes immutable member roots;
    // each map below is now extensional in its input.
    let f_map: BTreeMap<_, _> = held
        .iter()
        .copied()
        .map(|root| {
            let flipped = arena.flip(root, 0);
            (root, [root, flipped].into_iter().collect::<Held<_>>())
        })
        .collect();
    let first_expected: Held<_> = f_map.values().flat_map(|ys| ys.iter().copied()).collect();
    let g_map: BTreeMap<_, _> = first_expected
        .iter()
        .copied()
        .map(|root| (root, unit(arena.flip(root, 1))))
        .collect();

    for (&root, outputs) in &f_map {
        assert_eq!(powerset_bind(unit(root), &f_map), *outputs);
    }
    assert_eq!(
        powerset_bind(
            held.clone(),
            &held.iter().copied().map(|x| (x, unit(x))).collect()
        ),
        held
    );
    let first = powerset_bind(held.clone(), &f_map);
    assert_eq!(first, first_expected);
    assert_eq!(first.len(), 4);
    let mapped_families: Held<Held<_>> = held.iter().map(|x| f_map[x].clone()).collect();
    assert_eq!(first, multiply(mapped_families));
    let constant: BTreeMap<_, _> = held.iter().copied().map(|x| (x, unit(original))).collect();
    assert_eq!(powerset_bind(held.clone(), &constant), unit(original));
    let left = powerset_bind(first, &g_map);
    let composite: BTreeMap<_, _> = f_map
        .iter()
        .map(|(&x, fx)| (x, powerset_bind(fx.clone(), &g_map)))
        .collect();
    let right = powerset_bind(held.clone(), &composite);
    assert_eq!(left, right);
    assert!(powerset_bind(Held::<usize>::new(), &g_map).is_empty());

    let nested = unit(unit(held.clone()));
    let separated: Held<Held<_>> = held.iter().copied().map(unit).collect();
    let grouped = unit(held.clone());
    assert_ne!(separated, grouped);
    assert_eq!(multiply(separated), multiply(grouped));
    assert_eq!(multiply(unit(held.clone())), held);
    assert_eq!(multiply(held.iter().copied().map(unit).collect()), held);
    assert_eq!(
        multiply(multiply(nested.clone())),
        multiply(nested.into_iter().map(multiply).collect())
    );

    for root in held {
        let intermediate = arena.flip(root, 0);
        let recovered = arena.flip(intermediate, 0);
        assert_eq!(recovered, root);
    }
}
