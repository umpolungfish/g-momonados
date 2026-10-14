//! The graph evaluator shared by host instruments and GPU parity checks.
use alloc::{vec, vec::Vec};
use crate::{check::Graph, classic::Token, imasm16_3::Reg16_3 as Val};

pub fn gate_out(tok: Token, x: Val, seed: Val, slot: usize, fan: usize) -> Val {
    match tok {
        Token::Vinit => seed,
        t if t.is_brancher() => {
            if fan >= 3 {
                match slot {
                    0 => x.constructive_part().truth_part(),
                    1 => x.constructive_part().falsity_part(),
                    _ => x.info_part(),
                }
            } else if slot == 0 { x.truth_part() } else { x.falsity_part() }
        }
        Token::Evalt => x.truth_part(),
        Token::Evalf => x.falsity_part(),
        Token::Arev => x.invol(),
        _ => x,
    }
}

/// Kleene iteration over the full carrier, including all three fork arms.
pub fn flow_values(g: &Graph, seed: Val) -> (Vec<Val>, Vec<Val>) {
    let mut edge_val = vec![Val::default(); g.edges.len()];
    let mut node_in = vec![Val::default(); g.nodes.len()];
    for _ in 0..4*g.edges.len().max(1)+4 {
        let mut changed = false;
        for i in 0..g.nodes.len() {
            let mut inp = Val::default();
            for (eidx, &(_, b)) in g.edges.iter().enumerate() {
                if b == i { inp = inp.union(edge_val[eidx]); }
            }
            node_in[i] = inp;
            let fan = g.out_degree(i);
            let mut slot = 0;
            for (eidx, &(a, _)) in g.edges.iter().enumerate() {
                if a == i {
                    let v = gate_out(g.nodes[i], inp, seed, slot, fan);
                    if edge_val[eidx] != v { edge_val[eidx] = v; changed = true; }
                    slot += 1;
                }
            }
        }
        if !changed { break; }
    }
    (node_in, edge_val)
}
