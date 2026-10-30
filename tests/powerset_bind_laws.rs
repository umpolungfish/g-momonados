use g_momonados::reentry_powerset::{powerset_bind, state_bind, state_unit, unit, Held};
use std::collections::BTreeMap;

#[test]
fn finite_powerset_laws_hold_for_every_arrow_pair_on_three_elements() {
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
                let composite: BTreeMap<_, _> =
                    f.iter().map(|(&x, ys)| (x, apply(ys.clone(), g))).collect();
                assert_eq!(apply(first.clone(), g), apply(set.clone(), &composite));
            }
        }
    }
}

#[test]
fn complete_state_is_part_of_each_powerset_member() {
    let initial = state_unit(3u8, 5u8);
    let f = BTreeMap::from([((5u8, 3u8), [(8u8, 4u8), (2, 5)].into_iter().collect())]);
    let g = BTreeMap::from([
        ((8u8, 4u8), unit((12u8, 8u8))),
        ((2u8, 5u8), unit((7u8, 10u8))),
    ]);
    for (&(value, state), output) in &f {
        assert_eq!(state_bind(state_unit(state, value), &f), *output);
    }
    let left = state_bind(state_bind(initial.clone(), &f), &g);
    let composite = BTreeMap::from([((5u8, 3u8), state_bind(f[&(5, 3)].clone(), &g))]);
    assert_eq!(left, state_bind(initial.clone(), &composite));
    let expanded = state_bind(initial.clone(), &f);
    let identity_on_expanded: BTreeMap<_, _> = expanded
        .iter()
        .map(|&(value, state)| ((value, state), unit((value, state))))
        .collect();
    assert_eq!(
        state_bind(expanded.clone(), &identity_on_expanded),
        expanded
    );
    let identity = BTreeMap::from([((5u8, 3u8), state_unit(3u8, 5u8))]);
    assert_eq!(state_bind(initial.clone(), &identity), initial);
    assert!(state_bind(Held::<(u8, u8)>::new(), &f).is_empty());
}

#[test]
fn bind_unions_outputs_and_treats_missing_table_entries_as_empty() {
    let input: Held<u8> = [0, 1, 2].into_iter().collect();
    let relation = BTreeMap::from([(0u8, unit(9u8)), (1u8, unit(9u8))]);
    assert_eq!(powerset_bind(input, &relation), unit(9));
}
