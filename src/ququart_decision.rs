//! Shared complex-amplitude decision branches for a coherent work register.
//! Equal branches share storage; gates act on the diagram without enumerating residues.
use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
use alloc::{collections::BTreeMap, vec::Vec};
use num_bigint::{BigInt, BigUint};
use num_traits::Zero;

#[cfg(feature = "hosted")]
type NodeIndex = std::collections::HashMap<Node, usize>;
#[cfg(not(feature = "hosted"))]
type NodeIndex = BTreeMap<Node, usize>;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    fn nested_arithmetic_preserves_every_complex_basis_coordinate() {
        let mut arena = DecisionArena::new(7);
        let z = arena.zero();
        let mut original = z;
        for value in 0u64..128 {
            let basis = arena.basis(&BigUint::from(value),BigInt::from(value+1));
            // Give every address a distinct non-real amplitude.
            let mut imaginary = arena.intern(Node::Leaf(BigInt::zero(),BigInt::from(2*value+7)));
            for wire in (0..7).rev() {
                imaginary = if value & (1 << wire) != 0 { arena.branch(wire,z,imaginary) }
                    else { arena.branch(wire,imaginary,z) };
            }
            let basis = arena.sum(basis,imaginary);
            original = arena.sum(original,basis);
        }
        let mut roots = [original,z,z,z,z];
        arena.fold(&mut roots);
        let mass = arena.mass(original);
        let register = [1usize,3,4];
        let controls = [(0usize,false),(2usize,true)];
        for constant in 0u64..8 {
            roots[1] = arena.add_constant(original,&register,&BigUint::from(constant),&controls);
            roots[2] = arena.compare_constant(original,&register,&BigUint::from(constant),&controls,6);
            arena.fold(&mut roots);
            for input in 0u64..128 {
                let enabled = input & 1 == 0 && input & 4 != 0;
                let digit = register.iter().enumerate().fold(0u64,|d,(bit,&wire)| d | (((input >> wire)&1)<<bit));
                let next_digit = (digit+constant)&7;
                let added = if enabled { register.iter().enumerate().fold(input,|v,(bit,&wire)|
                    (v & !(1 << wire)) | (((next_digit >> bit)&1)<<wire)) } else { input };
                let compared = if enabled && digit < constant { input ^ 64 } else { input };
                let expected = (BigInt::from(input+1),BigInt::from(2*input+7));
                assert_eq!(arena.amplitude(roots[1],&BigUint::from(added)),expected);
                assert_eq!(arena.amplitude(roots[2],&BigUint::from(compared)),expected);
            }
            assert_eq!(arena.mass(roots[1]),mass);
            assert_eq!(arena.mass(roots[2]),mass);
        }
    }

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
    unique: NodeIndex,
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
            unique: NodeIndex::new(),
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

    pub fn controls_possible(&self, root: usize, controls: &[(usize, bool)]) -> bool {
        let fixed = &self.fixed[root];
        !fixed.empty && !controls.iter().any(|&(wire, positive)| {
            if positive { fixed.zero.bit(wire as u64) } else { fixed.one.bit(wire as u64) }
        })
    }

    /// Fuse changed and unchanged arms on the original literal controls.
    pub fn conditional_literals(&mut self, a: usize, b: usize, controls: &[(usize, bool)]) -> usize {
        fn visit(arena: &mut DecisionArena, a: usize, b: usize,
            controls: &[(usize, bool)], at: usize,
            memo: &mut BTreeMap<(usize, usize, usize), usize>) -> usize {
            if a == b { return a; }
            if at == controls.len() { return b; }
            if let Some(&result) = memo.get(&(a,b,at)) { return result; }
            let (control, positive) = controls[at];
            let wire = arena.top(a).into_iter().chain(arena.top(b))
                .chain(core::iter::once(control)).min().unwrap();
            let (al,ah) = arena.split(a,wire);
            let (bl,bh) = arena.split(b,wire);
            let (low,high) = if wire == control {
                if positive { (al,visit(arena,ah,bh,controls,at+1,memo)) }
                else { (visit(arena,al,bl,controls,at+1,memo),ah) }
            } else {
                (visit(arena,al,bl,controls,at,memo),visit(arena,ah,bh,controls,at,memo))
            };
            let result = arena.branch(wire,low,high);
            memo.insert((a,b,at),result);
            result
        }
        visit(self,a,b,controls,0,&mut BTreeMap::new())
    }

    /// Reversible constant addition as a shared dyadic split/fuse transducer.
    /// Output coordinates select the original input by the subtraction borrow.
    /// The final borrow is discarded only at the register's modular boundary.
    pub fn add_constant(&mut self, root: usize, register: &[usize], value: &BigUint,
        controls: &[(usize, bool)]) -> usize {
        if !self.controls_possible(root,controls) || value.is_zero() { return root; }
        fn visit(arena: &mut DecisionArena, id: usize, register: &[usize], value: &BigUint,
            at: usize, borrow: bool, memo: &mut BTreeMap<(usize,usize,bool),usize>) -> usize {
            if at == register.len() || matches!(arena.node(id),Node::Leaf(..)) { return id; }
            if let Some(&result) = memo.get(&(id,at,borrow)) { return result; }
            let wire = arena.top(id).unwrap().min(register[at]);
            let (low,high) = arena.split(id,wire);
            let (low,high) = if wire == register[at] {
                let constant = value.bit(at as u64);
                let (for_zero,for_one) = if constant ^ borrow { (high,low) } else { (low,high) };
                (visit(arena,for_zero,register,value,at+1,constant || borrow,memo),
                 visit(arena,for_one,register,value,at+1,constant && borrow,memo))
            } else {
                (visit(arena,low,register,value,at,borrow,memo),
                 visit(arena,high,register,value,at,borrow,memo))
            };
            let result = arena.branch(wire,low,high);
            memo.insert((id,at,borrow),result);
            result
        }
        // Split on the outer controls before doing arithmetic. Disabled arms
        // retain their original amplitudes and never enter the transducer.
        let zero = self.zero();
        let enabled = self.conditional_literals(zero,root,controls);
        let changed = visit(self,enabled,register,value,0,false,&mut BTreeMap::new());
        self.conditional_literals(root,changed,controls)
    }

    /// XOR a high flag with the exact less-than predicate on a register.
    /// Each more significant bit updates the comparison carried by its inner
    /// dyad; the flag is transformed after the complete register has fused.
    pub fn compare_constant(&mut self, root: usize, register: &[usize], value: &BigUint,
        controls: &[(usize,bool)], flag: usize) -> usize {
        if !self.controls_possible(root,controls) || value.is_zero() { return root; }
        fn visit(arena: &mut DecisionArena,id: usize,register: &[usize],value: &BigUint,
            at: usize,less: bool,flag: usize,memo: &mut BTreeMap<(usize,usize,bool),usize>) -> usize {
            if at == register.len() { return if less { arena.flip(id,flag) } else { id }; }
            if matches!(arena.node(id),Node::Leaf(..)) { return id; }
            if let Some(&result) = memo.get(&(id,at,less)) { return result; }
            let wire = arena.top(id).unwrap().min(register[at]);
            let (low,high) = arena.split(id,wire);
            let (low,high) = if wire == register[at] {
                let constant = value.bit(at as u64);
                (visit(arena,low,register,value,at+1,constant || less,flag,memo),
                 visit(arena,high,register,value,at+1,constant && less,flag,memo))
            } else {
                (visit(arena,low,register,value,at,less,flag,memo),
                 visit(arena,high,register,value,at,less,flag,memo))
            };
            let result = arena.branch(wire,low,high);
            memo.insert((id,at,less),result);
            result
        }
        let zero = self.zero();
        let enabled = self.conditional_literals(zero,root,controls);
        let changed = visit(self,enabled,register,value,0,false,flag,&mut BTreeMap::new());
        self.conditional_literals(root,changed,controls)
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
