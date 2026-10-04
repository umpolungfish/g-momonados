//! Shared complex-amplitude decision branches for a coherent work register.
//! Equal branches share storage; gates act on the diagram without enumerating residues.
use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
use alloc::{collections::BTreeMap, vec::Vec};
use num_bigint::{BigInt, BigUint};
use num_traits::Zero;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Node {
    Leaf(BigInt, BigInt),
    Branch {
        wire: usize,
        low: usize,
        high: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::One;

    #[test]
    fn controlled_permutations_preserve_complex_support_through_slot_reuse() {
        let n = BigUint::parse_bytes(
            b"307795685303946736105416402281159953456744654179248447762559917", 10,
        ).unwrap();
        let x = &n - 1u8;
        let y = &n - 2u8;
        let mut arena = DecisionArena::new(400);
        let z = arena.zero();
        let right = arena.basis(&y, BigInt::from(7));
        // Use distinct exact complex coordinates without a fixed-point rounding seam.
        let amplitude = arena.intern(Node::Leaf(BigInt::from(3), BigInt::from(5)));
        let mut complex_left = amplitude;
        for wire in (0..400).rev() {
            complex_left = if x.bit(wire as u64) { arena.branch(wire, z, complex_left) }
                else { arena.branch(wire, complex_left, z) };
        }
        let root = arena.sum(complex_left, right);
        let mut roots = [root, z, z, z, z];
        arena.fold(&mut roots);
        let mut expected = BTreeMap::from([
            (x, (BigInt::from(3), BigInt::from(5))),
            (y, (BigInt::from(7), BigInt::zero())),
        ]);
        for controls in [vec![300], vec![207], vec![0, 207], vec![0, 207], vec![207]] {
            roots[0] = arena.controlled_flip(roots[0], 100, &controls);
            arena.fold(&mut roots);
            expected = expected.into_iter().map(|(mut value, amplitude)| {
                if controls.iter().all(|&wire| value.bit(wire as u64)) {
                    value ^= BigUint::one() << 100usize;
                }
                (value, amplitude)
            }).collect();
            for (value, amplitude) in &expected {
                assert_eq!(&arena.amplitude(roots[0], value), amplitude);
            }
            assert_eq!(arena.mass(roots[0]), BigUint::from(83u8));
            assert!(arena.mass(roots[1]).is_zero());
        }
    }
}
/// Bits fixed on every basis state with a nonzero complex amplitude.
/// This is exact support information, independent of amplitude magnitude.
#[derive(Clone)]
struct FixedBits {
    empty: bool,
    zero: BigUint,
    one: BigUint,
}
impl FixedBits {
    fn leaf(empty: bool) -> Self {
        Self { empty, zero: BigUint::zero(), one: BigUint::zero() }
    }
}
pub struct DecisionArena {
    nodes: Vec<Option<Node>>,
    fixed: Vec<FixedBits>,
    references: Vec<usize>,
    free: Vec<usize>,
    created: Vec<usize>,
    pinned: Option<[usize; 5]>,
    unique: BTreeMap<Node, usize>,
    pub cells: usize,
}
impl DecisionArena {
    pub fn new(cells: usize) -> Self {
        Self {
            nodes: Vec::new(),
            fixed: Vec::new(),
            references: Vec::new(),
            free: Vec::new(),
            created: Vec::new(),
            pinned: None,
            unique: BTreeMap::new(),
            cells,
        }
    }
    fn node(&self, id: usize) -> &Node {
        self.nodes[id].as_ref().unwrap()
    }
    fn intern(&mut self, node: Node) -> usize {
        if let Some(id) = self.unique.get(&node) {
            return *id;
        }
        let fixed = match &node {
            Node::Leaf(re, im) => FixedBits::leaf(re.is_zero() && im.is_zero()),
            Node::Branch { wire, low, high } => {
                let low = &self.fixed[*low];
                let high = &self.fixed[*high];
                let mut fixed = match (low.empty, high.empty) {
                    (true, true) => FixedBits::leaf(true),
                    (false, true) => low.clone(),
                    (true, false) => high.clone(),
                    (false, false) => FixedBits {
                        empty: false,
                        zero: &low.zero & &high.zero,
                        one: &low.one & &high.one,
                    },
                };
                if !fixed.empty {
                    fixed.zero.set_bit(*wire as u64, high.empty);
                    fixed.one.set_bit(*wire as u64, low.empty);
                }
                fixed
            }
        };
        if let Node::Branch { low, high, .. } = &node {
            self.references[*low] += 1;
            self.references[*high] += 1;
        }
        let id = if let Some(id) = self.free.pop() {
            self.nodes[id] = Some(node.clone());
            self.fixed[id] = fixed;
            self.references[id] = 0;
            id
        } else {
            let id = self.nodes.len();
            self.nodes.push(Some(node.clone()));
            self.fixed.push(fixed);
            self.references.push(0);
            id
        };
        self.unique.insert(node, id);
        self.created.push(id);
        id
    }
    fn release(&mut self, id: usize) {
        self.references[id] -= 1;
        if self.references[id] == 0 {
            self.reclaim(id);
        }
    }
    fn reclaim(&mut self, id: usize) {
        let Some(node) = self.nodes[id].take() else {
            return;
        };
        self.unique.remove(&node);
        self.fixed[id] = FixedBits::leaf(true);
        self.free.push(id);
        if let Node::Branch { low, high, .. } = node {
            self.release(low);
            self.release(high);
        }
    }
    pub fn zero(&mut self) -> usize {
        self.intern(Node::Leaf(BigInt::zero(), BigInt::zero()))
    }
    fn branch(&mut self, wire: usize, low: usize, high: usize) -> usize {
        if low == high {
            low
        } else {
            self.intern(Node::Branch { wire, low, high })
        }
    }
    fn top(&self, id: usize) -> Option<usize> {
        match *self.node(id) {
            Node::Branch { wire, .. } => Some(wire),
            _ => None,
        }
    }
    fn split(&self, id: usize, at: usize) -> (usize, usize) {
        match *self.node(id) {
            Node::Branch { wire, low, high } if wire == at => (low, high),
            _ => (id, id),
        }
    }
    pub fn basis(&mut self, value: &BigUint, scale: BigInt) -> usize {
        let z = self.zero();
        let mut root = self.intern(Node::Leaf(scale, BigInt::zero()));
        for wire in (0..self.cells).rev() {
            root = if value.bit(wire as u64) {
                self.branch(wire, z, root)
            } else {
                self.branch(wire, root, z)
            };
        }
        root
    }
    pub fn scale(&mut self, id: usize, scalar: &FixedComplex, format: &FixedPointFormat) -> usize {
        if scalar.re.is_zero() && scalar.im.is_zero() {
            return self.zero();
        }
        fn visit(
            a: &mut DecisionArena,
            id: usize,
            scalar: &FixedComplex,
            f: &FixedPointFormat,
            memo: &mut BTreeMap<usize, usize>,
        ) -> usize {
            if let Some(r) = memo.get(&id) {
                return *r;
            }
            let r = match a.node(id).clone() {
                Node::Leaf(re, im) => {
                    let z = FixedComplex { re, im }.mul(scalar, f);
                    a.intern(Node::Leaf(z.re, z.im))
                }
                Node::Branch { wire, low, high } => {
                    let low = visit(a, low, scalar, f, memo);
                    let high = visit(a, high, scalar, f, memo);
                    a.branch(wire, low, high)
                }
            };
            memo.insert(id, r);
            r
        }
        visit(self, id, scalar, format, &mut BTreeMap::new())
    }
    pub fn sum(&mut self, a: usize, b: usize) -> usize {
        if let Node::Leaf(re, im) = self.node(a) {
            if re.is_zero() && im.is_zero() {
                return b;
            }
        }
        if let Node::Leaf(re, im) = self.node(b) {
            if re.is_zero() && im.is_zero() {
                return a;
            }
        }
        fn visit(
            arena: &mut DecisionArena,
            a: usize,
            b: usize,
            memo: &mut BTreeMap<(usize, usize), usize>,
        ) -> usize {
            if let Some(r) = memo.get(&(a, b)) {
                return *r;
            }
            let r = match (arena.top(a), arena.top(b)) {
                (None, None) => {
                    let (Node::Leaf(ar, ai), Node::Leaf(br, bi)) = (arena.node(a), arena.node(b))
                    else {
                        unreachable!()
                    };
                    arena.intern(Node::Leaf(ar + br, ai + bi))
                }
                (x, y) => {
                    let wire = x.into_iter().chain(y).min().unwrap();
                    let (al, ah) = arena.split(a, wire);
                    let (bl, bh) = arena.split(b, wire);
                    let low = visit(arena, al, bl, memo);
                    let high = visit(arena, ah, bh, memo);
                    arena.branch(wire, low, high)
                }
            };
            memo.insert((a, b), r);
            r
        }
        visit(self, a, b, &mut BTreeMap::new())
    }
    pub fn flip(&mut self, id: usize, target: usize) -> usize {
        fn visit(
            a: &mut DecisionArena,
            id: usize,
            t: usize,
            memo: &mut BTreeMap<usize, usize>,
        ) -> usize {
            if let Some(r) = memo.get(&id) {
                return *r;
            }
            let r = match a.node(id).clone() {
                Node::Branch { wire, low, high } if wire == t => a.branch(wire, high, low),
                Node::Branch { wire, low, high } if wire < t => {
                    let low = visit(a, low, t, memo);
                    let high = visit(a, high, t, memo);
                    a.branch(wire, low, high)
                }
                _ => id,
            };
            memo.insert(id, r);
            r
        }
        visit(self, id, target, &mut BTreeMap::new())
    }
    /// Apply a controlled work-bit permutation directly to shared branches.
    /// Constant subtrees remain unchanged, including every zero subtree.
    pub fn controlled_flip(&mut self, root: usize, target: usize, controls: &[usize]) -> usize {
        let support = &self.fixed[root];
        if support.empty || controls.iter().any(|&wire| support.zero.bit(wire as u64)) {
            return root;
        }
        let controls: Vec<_> = controls.iter().copied()
            .filter(|&wire| !support.one.bit(wire as u64)).collect();
        if controls.is_empty() {
            return self.flip(root, target);
        }
        fn visit(
            a: &mut DecisionArena,
            id: usize,
            target: usize,
            controls: &[usize],
            at: usize,
            memo: &mut BTreeMap<(usize, usize), usize>,
        ) -> usize {
            if matches!(a.node(id), Node::Leaf(..)) {
                return id;
            }
            if let Some(r) = memo.get(&(id, at)) {
                return *r;
            }
            if at == controls.len() {
                return a.flip(id, target);
            }
            let wire = a.top(id).unwrap();
            let control = controls[at];
            let r = if control < wire {
                let high = visit(a, id, target, controls, at + 1, memo);
                a.branch(control, id, high)
            } else {
                let (low, high) = a.split(id, wire);
                if wire == control {
                    let high = visit(a, high, target, controls, at + 1, memo);
                    a.branch(wire, low, high)
                } else if wire == target {
                    let low_new = a.conditional(low, high, &controls[at..]);
                    let high_new = a.conditional(high, low, &controls[at..]);
                    a.branch(wire, low_new, high_new)
                } else {
                    let low = visit(a, low, target, controls, at, memo);
                    let high = visit(a, high, target, controls, at, memo);
                    a.branch(wire, low, high)
                }
            };
            memo.insert((id, at), r);
            r
        }
        visit(self, root, target, &controls, 0, &mut BTreeMap::new())
    }
    pub fn conditional(&mut self, a: usize, b: usize, controls: &[usize]) -> usize {
        fn visit(
            arena: &mut DecisionArena,
            a: usize,
            b: usize,
            controls: &[usize],
            at: usize,
            memo: &mut BTreeMap<(usize, usize, usize), usize>,
        ) -> usize {
            if a == b {
                return a;
            }
            if at == controls.len() {
                return b;
            }
            if let Some(r) = memo.get(&(a, b, at)) {
                return *r;
            }
            let c = controls[at];
            let wire = arena
                .top(a)
                .into_iter()
                .chain(arena.top(b))
                .chain(core::iter::once(c))
                .min()
                .unwrap();
            let (al, ah) = arena.split(a, wire);
            let (bl, bh) = arena.split(b, wire);
            let (low, high) = if wire == c {
                (al, visit(arena, ah, bh, controls, at + 1, memo))
            } else {
                (
                    visit(arena, al, bl, controls, at, memo),
                    visit(arena, ah, bh, controls, at, memo),
                )
            };
            let r = arena.branch(wire, low, high);
            memo.insert((a, b, at), r);
            r
        }
        visit(self, a, b, controls, 0, &mut BTreeMap::new())
    }
    pub fn mass(&self, root: usize) -> BigUint {
        fn visit(
            a: &DecisionArena,
            id: usize,
            depth: usize,
            memo: &mut BTreeMap<(usize, usize), BigUint>,
        ) -> BigUint {
            if let Some(r) = memo.get(&(id, depth)) {
                return r.clone();
            }
            let r = match a.node(id) {
                Node::Leaf(re, im) => {
                    (re * re + im * im).to_biguint().unwrap() << (a.cells - depth)
                }
                &Node::Branch { wire, low, high } => {
                    (visit(a, low, wire + 1, memo) + visit(a, high, wire + 1, memo))
                        << (wire - depth)
                }
            };
            memo.insert((id, depth), r.clone());
            r
        }
        visit(self, root, 0, &mut BTreeMap::new())
    }
    pub fn normalize(&mut self, root: usize, norm: &BigInt, scale: &BigInt) -> usize {
        fn visit(
            a: &mut DecisionArena,
            id: usize,
            norm: &BigInt,
            scale: &BigInt,
            memo: &mut BTreeMap<usize, usize>,
        ) -> usize {
            if let Some(r) = memo.get(&id) {
                return *r;
            }
            let r = match a.node(id).clone() {
                Node::Leaf(re, im) => a.intern(Node::Leaf(re * scale / norm, im * scale / norm)),
                Node::Branch { wire, low, high } => {
                    let low = visit(a, low, norm, scale, memo);
                    let high = visit(a, high, norm, scale, memo);
                    a.branch(wire, low, high)
                }
            };
            memo.insert(id, r);
            r
        }
        visit(self, root, norm, scale, &mut BTreeMap::new())
    }
    /// Pin the new roots, release only unreachable branches, and reuse slots.
    /// No complete diagram copy or node-count/heap cutoff is involved.
    pub fn fold(&mut self, roots: &mut [usize; 5]) {
        for &root in roots.iter() {
            self.references[root] += 1;
        }
        if let Some(old) = self.pinned.replace(*roots) {
            for root in old {
                self.release(root);
            }
        }
        let created = core::mem::take(&mut self.created);
        for id in created {
            if self.references[id] == 0 {
                self.reclaim(id);
            }
        }
    }
    pub fn retained_nodes(&self) -> usize {
        self.nodes.len() - self.free.len()
    }
    #[cfg(test)]
    pub fn amplitude(&self, mut root: usize, value: &BigUint) -> (BigInt, BigInt) {
        loop {
            match self.node(root) {
                Node::Leaf(re, im) => return (re.clone(), im.clone()),
                &Node::Branch { wire, low, high } => {
                    root = if value.bit(wire as u64) { high } else { low }
                }
            }
        }
    }
}
