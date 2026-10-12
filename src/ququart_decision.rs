//! Shared complex-amplitude decision branches for a coherent work register.
//! Equal branches share storage; gates act on the diagram without enumerating residues.
use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
use alloc::{sync::Arc, vec::Vec};
#[cfg(any(test, not(feature = "hosted")))]
use alloc::collections::BTreeMap;
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Zero};
use core::sync::atomic::{AtomicU64, Ordering};

// Native Vox reads these while the process is stopped. They do not enter
// arithmetic, evidence, or the terminal readout.
#[cfg_attr(not(test), no_mangle)]
pub static VOX_MODULAR_COUNTERS: [AtomicU64; 4] = [
    AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0),
];

#[cfg(feature = "hosted")]
type OperationCache<K, V> = std::collections::HashMap<K, V,
    core::hash::BuildHasherDefault<CoordinateHasher>>;
#[cfg(not(feature = "hosted"))]
type OperationCache<K, V> = BTreeMap<K, V>;

// Only internal arena coordinates use this index. Equality still compares both
// child IDs; hashing never decides whether two coherent branches are equal.
#[cfg(feature = "hosted")]
#[derive(Default)]
struct CoordinateHasher(u64);
#[cfg(feature = "hosted")]
impl core::hash::Hasher for CoordinateHasher {
    fn finish(&self) -> u64 { self.0 }
    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes { self.write_u64(byte as u64); }
    }
    fn write_usize(&mut self, value: usize) { self.write_u64(value as u64); }
    fn write_u64(&mut self, value: u64) {
        let mut mixed = value.wrapping_add(self.0).wrapping_add(0x9e3779b97f4a7c15);
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d049bb133111eb);
        self.0 = mixed ^ (mixed >> 31);
    }
}
#[cfg(feature = "hosted")]
type BranchCache = std::collections::HashMap<(usize,usize),usize,
    core::hash::BuildHasherDefault<CoordinateHasher>>;
#[cfg(not(feature = "hosted"))]
type BranchCache = BTreeMap<(usize,usize),usize>;

// Branch lookups use only their three coordinates. Amplitude leaves retain
// their full exact integers in a separate index instead of inflating every
// branch hash-table entry with the leaf enum's storage.
#[cfg(feature = "hosted")]
type LeafCache = std::collections::HashMap<Node,usize,
    core::hash::BuildHasherDefault<LeafHasher>>;
#[cfg(not(feature = "hosted"))]
type LeafCache = BTreeMap<Node,usize>;

// Leaf values can contain thousands of exact bits. Use a small deterministic
// byte hasher for the disposable interning index; equality still compares the
// complete BigInts, so collisions cannot merge distinct amplitudes.
#[cfg(feature = "hosted")]
struct LeafHasher(u64);
#[cfg(feature = "hosted")]
impl Default for LeafHasher {
    fn default() -> Self { Self(0xcbf29ce484222325) }
}
#[cfg(feature = "hosted")]
impl core::hash::Hasher for LeafHasher {
    fn finish(&self) -> u64 { self.0 }
    fn write(&mut self, bytes: &[u8]) {
        let mut hash = self.0;
        for &byte in bytes {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
        self.0 = hash;
    }
}

struct NodeIndex {
    branches: Vec<BranchCache>,
    leaves: LeafCache,
}
impl NodeIndex {
    fn new() -> Self {
        Self { branches: Vec::new(), leaves: LeafCache::default() }
    }
    fn get(&self, node: &Node) -> Option<&usize> {
        match node {
            Node::Branch { wire, low, high } => self.branches.get(*wire)?.get(&(*low,*high)),
            Node::Leaf(..) => self.leaves.get(node),
        }
    }
    fn insert(&mut self, node: Node, id: usize) {
        match node {
            Node::Branch { wire, low, high } => {
                if self.branches.len() <= wire {
                    self.branches.resize_with(wire+1, BranchCache::default);
                }
                self.branches[wire].insert((low,high),id);
            }
            Node::Leaf(..) => { self.leaves.insert(node,id); }
        }
    }
    fn remove(&mut self, node: &Node) {
        match node {
            Node::Branch { wire, low, high } => {
                if let Some(index) = self.branches.get_mut(*wire) { index.remove(&(*low,*high)); }
            }
            Node::Leaf(..) => { self.leaves.remove(node); }
        }
    }
}

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

    fn rsa_arena() -> DecisionArena {
        let source = BigUint::parse_bytes(
            b"1156514714917773145849996001252587703581994899993461612691909", 10).unwrap();
        assert!(source.bits() >= 200);
        DecisionArena::new(source.bits() as usize + 2)
    }

    #[test]
    fn whole_register_readout_keeps_interference_and_marginalizes_work_wires() {
        let source = BigUint::from(15u8);
        let format = FixedPointFormat::for_modulus(&source).unwrap();
        let mut arena = DecisionArena::new(4);
        let mut root = arena.basis(&BigUint::zero(), format.scale());
        root = arena.hadamard(root, 0, &format).unwrap();
        root = arena.hadamard(root, 1, &format).unwrap();
        root = arena.controlled_winding(root, &[(0, true), (1, true)],
            &BigInt::one(), &BigUint::from(2u8), &format).unwrap();
        root = arena.hadamard(root, 0, &format).unwrap();
        root = arena.hadamard(root, 1, &format).unwrap();

        let denominator = BigUint::from(10u8);
        assert_eq!(arena.sample_prefix(root, 2, &BigUint::one(), &denominator).unwrap(), BigUint::zero());
        assert_eq!(arena.sample_prefix(root, 2, &BigUint::from(9u8), &denominator).unwrap(), BigUint::from(3u8));
    }

    #[test]
    fn whole_register_readout_rejects_partial_or_out_of_range_requests() {
        let source = BigUint::from(15u8);
        let format = FixedPointFormat::for_modulus(&source).unwrap();
        let mut arena = DecisionArena::new(2);
        let root = arena.basis(&BigUint::zero(), format.scale());
        assert!(arena.hadamard(root, 2, &format).is_err());
        assert!(arena.sample_prefix(root, 3, &BigUint::zero(), &BigUint::one()).is_err());
        assert!(arena.sample_prefix(root, 2, &BigUint::one(), &BigUint::one()).is_err());
    }

    #[test]
    fn full_fourier_alignment_round_trips_one_joint_basis_register() {
        let source = BigUint::from(15u8);
        let format = FixedPointFormat::for_modulus(&source).unwrap();
        let mut arena = DecisionArena::new(3);
        let root = arena.basis(&BigUint::from(5u8), format.scale());
        let wires = [0, 1, 2];
        let transformed = arena.fourier_register(root, &wires, false, &format).unwrap();
        let recovered = arena.fourier_register(transformed, &wires, true, &format).unwrap();
        assert_eq!(arena.sample_prefix(recovered, 3, &BigUint::one(), &BigUint::from(2u8)).unwrap(), BigUint::from(5u8));
    }

    #[test]
    #[should_panic(expected = "transformed source arms did not recover")]
    fn rsa_source_fuse_rejects_an_arm_with_its_source_still_attached() {
        let mut arena = rsa_arena();
        let low = arena.basis(&BigUint::zero(), BigInt::from(3));
        let high = arena.basis(&BigUint::one(), BigInt::from(7));
        arena.fuse_source_arms(low, high, 0);
    }

    #[test]
    #[should_panic(expected = "transformed interval arms did not recover")]
    fn rsa_interval_fuse_rejects_overlapping_transformed_arms() {
        let mut arena = rsa_arena();
        let arm = arena.basis(&BigUint::one(), BigInt::from(11));
        let register: Vec<_> = (0..arena.cells).collect();
        arena.fuse_interval_arms(arm, arm, &register, &BigUint::from(2u8));
    }

    #[test]
    #[should_panic(expected = "split/fuse child escaped")]
    fn rsa_wire_fuse_rejects_a_child_above_its_parent() {
        let mut arena = rsa_arena();
        let child = arena.basis(&BigUint::one(), BigInt::from(11));
        let zero = arena.zero();
        arena.branch(1, child, zero);
    }

    #[test]
    fn rsa_200_and_256_bit_modular_translation_preserves_coherent_amplitudes() {
        for decimal in [
            "1156514714917773145849996001252587703581994899993461612691909",
            "101560191607051872909385412079844080615251494997013823952605603214371184345809",
        ] {
            let n = BigUint::parse_bytes(decimal.as_bytes(),10).unwrap();
            assert!(n.bits() >= 200);
            let width = n.bits() as usize + 1;
            let mut arena = DecisionArena::new(width + 1);
            let z = arena.zero();
            let scale = BigInt::one() << (2 * n.bits() + 16) as usize;
            let values = [BigUint::zero(),BigUint::one(),&n-BigUint::one(),n.clone(),&n+BigUint::one()];
            let mut root = z;
            let mut inputs = Vec::new();
            for enabled in [false,true] {
                for (index,value) in values.iter().enumerate() {
                    let address = if enabled { value | (BigUint::one() << width) } else { value.clone() };
                    let re = &scale * (index+1);
                    let im = -&scale * (index+2);
                    let mut basis = arena.intern(Node::Leaf(re.clone(),im.clone()));
                    for wire in (0..arena.cells).rev() {
                        basis = if address.bit(wire as u64) { arena.branch(wire,z,basis) }
                            else { arena.branch(wire,basis,z) };
                    }
                    root = arena.sum(root,basis);
                    inputs.push((enabled,value.clone(),re,im));
                }
            }
            let mut roots = [root,z,z,z,z];
            arena.fold(&mut roots);
            let mass = arena.mass(root);
            let register: Vec<_> = (0..width).collect();
            for constant in [BigUint::from(2u8),BigUint::from(8u8),&n-BigUint::from(2u8)] {
                roots[1] = arena.modular_add(root,&register,&constant,&n,&[(width,true)]);
                arena.fold(&mut roots);
                for (enabled,input,re,im) in &inputs {
                    let output = if *enabled && input < &n { (input+&constant)%&n } else { input.clone() };
                    let address = if *enabled { output | (BigUint::one()<<width) } else { output };
                    assert_eq!(arena.amplitude(roots[1],&address),(re.clone(),im.clone()));
                }
                assert_eq!(arena.mass(roots[1]),mass,"unitary modular translation preserves mass");
            }
        }
    }

    #[test]
    fn rsa_200_and_256_bit_modular_translation_closes_on_correlated_spectators() {
        for decimal in [
            "1156514714917773145849996001252587703581994899993461612691909",
            "101560191607051872909385412079844080615251494997013823952605603214371184345809",
        ] {
            let n = BigUint::parse_bytes(decimal.as_bytes(),10).unwrap();
            assert!(n.bits() >= 200);
            let width = n.bits() as usize + 1;
            for interleaved in [false,true] {
                let cells = if interleaved { 2*width+1 } else { width+2 };
                let register: Vec<usize> = (0..width).map(|bit|
                    if interleaved { 2*bit+1 } else { bit+1 }).collect();
                let spectator = cells-1;
                let address = |value:&BigUint, enabled:bool, tag:bool| {
                    let mut address = BigUint::from(u8::from(enabled));
                    for (bit,&wire) in register.iter().enumerate() {
                        address.set_bit(wire as u64,value.bit(bit as u64));
                    }
                    address.set_bit(spectator as u64,tag);
                    address
                };
                let mut arena = DecisionArena::new(cells);
                let z = arena.zero();
                let values = [BigUint::zero(),BigUint::one(),&n-1u8,n.clone(),&n+1u8,
                              (BigUint::one()<<width)-1u8];
                let mut root = z;
                let mut inputs = Vec::new();
                for enabled in [false,true] {
                    for tag in [false,true] {
                        for (index,value) in values.iter().enumerate() {
                            let re = BigInt::from(1+index+13*usize::from(enabled)+29*usize::from(tag));
                            let im = -BigInt::from(7+3*index+17*usize::from(enabled)+31*usize::from(tag));
                            let location = address(value,enabled,tag);
                            let mut basis = arena.intern(Node::Leaf(re.clone(),im.clone()));
                            for wire in (0..cells).rev() {
                                basis = if location.bit(wire as u64) {arena.branch(wire,z,basis)}
                                    else {arena.branch(wire,basis,z)};
                            }
                            root = arena.sum(root,basis);
                            inputs.push((value.clone(),enabled,tag,re,im));
                        }
                    }
                }
                let mut roots = [root,z,z,z,z];
                arena.fold(&mut roots);
                let mass = arena.mass(root);
                for value in [BigUint::from(2u8),&n/2u8,&n-2u8] {
                    let controls = [(0,true),(spectator,false)];
                    roots[1] = arena.modular_add(root,&register,&value,&n,&controls);
                    for (input,enabled,tag,re,im) in &inputs {
                        let output = if *enabled && !*tag && input < &n {(input+&value)%&n}
                            else {input.clone()};
                        assert_eq!(arena.amplitude(roots[1],&address(&output,*enabled,*tag)),
                            (re.clone(),im.clone()));
                    }
                    assert_eq!(arena.mass(roots[1]),mass);
                    roots[2] = arena.modular_add(roots[1],&register,&(&n-&value),&n,&controls);
                    assert_eq!(roots[2],root,"transformed correlated object must return exactly");
                    arena.fold(&mut roots);
                }
            }
        }
    }

    #[test]
    fn rsa_200_and_256_bit_reclamation_releases_masks_without_changing_live_support() {
        for decimal in [
            "1156514714917773145849996001252587703581994899993461612691909",
            "101560191607051872909385412079844080615251494997013823952605603214371184345809",
        ] {
            let n = BigUint::parse_bytes(decimal.as_bytes(),10).unwrap();
            assert!(n.bits() >= 200);
            let mut arena = DecisionArena::new(3*n.bits() as usize+3);
            let z = arena.zero();
            let address = &n-1u8;
            let root = arena.basis(&address, BigInt::from(7));
            let mut roots = [root,z,z,z,z];
            let created_capacity = arena.created.capacity();
            arena.fold(&mut roots);
            assert!(arena.created.is_empty());
            assert!(arena.created.capacity() >= created_capacity);
            assert_eq!(arena.amplitude(root,&address), (BigInt::from(7),BigInt::zero()));
            assert!(arena.empty_fixed.zero.is_zero());
            assert!(arena.empty_fixed.one.is_zero());
            roots = [z;5];
            arena.fold(&mut roots);
            assert!(!arena.free.is_empty());
            for &id in &arena.free {
                assert!(arena.nodes[id].is_none());
                assert!(arena.fixed[id].empty);
                assert!(arena.fixed[id].zero.0.is_none(),"vacant node retained zero-mask references");
                assert!(arena.fixed[id].one.0.is_none(),"vacant node retained one-mask references");
            }
            let root = arena.basis(&address, BigInt::from(11));
            assert_eq!(arena.amplitude(root,&address), (BigInt::from(11),BigInt::zero()));
            assert!(arena.empty_fixed.zero.is_zero());
            assert!(arena.empty_fixed.one.is_zero());
            for wire in n.bits() as usize..arena.cells {
                assert!(arena.wire_is_zero(root,wire),"reused slot lost an upper workspace constraint");
            }
        }
    }

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
    fn precomputed_modular_digit_add_preserves_controlled_complex_states() {
        let mut arena = DecisionArena::new(14);
        let zero = arena.zero();
        let mut root = zero;
        let mut cases = Vec::new();
        let mut ordinal = 0u64;
        for work in [0u64,1,172,173,200,511] {
            for digit in [0u64,1,7,15] {
                for enabled in [false,true] {
                    let address = BigUint::from(work) | (BigUint::from(digit) << 9usize)
                        | (BigUint::from(u8::from(enabled)) << 13usize);
                    let amplitude = (BigInt::from(ordinal+3),-BigInt::from(ordinal+11));
                    ordinal += 1;
                    let mut basis = arena.intern(Node::Leaf(amplitude.0.clone(),amplitude.1.clone()));
                    for wire in (0..arena.cells).rev() {
                        basis = if address.bit(wire as u64) { arena.branch(wire,zero,basis) }
                            else { arena.branch(wire,basis,zero) };
                    }
                    root = arena.sum(root,basis);
                    cases.push((work,digit,enabled,address,amplitude));
                }
            }
        }
        let mut roots = [root,zero,zero,zero,zero];
        arena.fold(&mut roots);
        let register: Vec<_> = (0..9).collect();
        let digit_wires: Vec<_> = (9..13).collect();
        let controls = [(13usize,true)];
        roots[1] = arena.modular_digit_add(root,&register,&digit_wires,
            &BigUint::from(17u8),&BigUint::from(173u16),&controls);
        arena.fold(&mut roots);
        for (work,digit,enabled,_,amplitude) in cases {
            let translated = if enabled && work < 173 { (work + digit*17) % 173 } else { work };
            let output = BigUint::from(translated) | (BigUint::from(digit) << 9usize)
                | (BigUint::from(u8::from(enabled)) << 13usize);
            assert_eq!(arena.amplitude(roots[1],&output),amplitude);
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
// Empty support masks are value zero, with no allocation or atomic references.
// Nonempty masks still share immutable storage and copy on mutation.
#[derive(Clone)]
struct SupportMask(Option<Arc<BigUint>>);
impl core::ops::Deref for SupportMask {
    type Target = BigUint;
    fn deref(&self) -> &BigUint {
        self.0.as_deref().unwrap_or(&BigUint::ZERO)
    }
}
impl SupportMask {
    fn intersection(&self, other: &Self) -> Self {
        let (Some(left),Some(right)) = (&self.0,&other.0) else { return Self(None); };
        if Arc::ptr_eq(left,right) { return self.clone(); }
        let value = &**left & &**right;
        if value.is_zero() { Self(None) } else { Self(Some(Arc::new(value))) }
    }
    fn set_bit(&mut self, wire: u64) {
        let storage = self.0.get_or_insert_with(|| Arc::new(BigUint::zero()));
        Arc::make_mut(storage).set_bit(wire,true);
    }
}

#[derive(Clone)]
struct FixedBits {
    empty: bool,
    zero: SupportMask,
    one: SupportMask,
}
impl FixedBits {
    fn leaf(empty: bool) -> Self {
        Self { empty, zero: SupportMask(None), one: SupportMask(None) }
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
    empty_fixed: FixedBits,
    pub cells: usize,
}

fn integer_sqrt(value: &BigUint) -> BigUint {
    if value < &BigUint::from(2u8) { return value.clone(); }
    let mut root = BigUint::one() << value.bits().div_ceil(2) as usize;
    loop {
        let next = (&root + value / &root) >> 1usize;
        if next >= root { return root; }
        root = next;
    }
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
            empty_fixed: FixedBits::leaf(true),
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
            Node::Leaf(re, im) => FixedBits {
                empty: re.is_zero() && im.is_zero(),
                zero: self.empty_fixed.zero.clone(),
                one: self.empty_fixed.one.clone(),
            },
            Node::Branch { wire, low, high } => {
                let low = &self.fixed[*low];
                let high = &self.fixed[*high];
                let mut fixed = match (low.empty, high.empty) {
                    (true, true) => self.empty_fixed.clone(),
                    (false, true) => FixedBits {
                        empty: false,
                        zero: low.zero.clone(),
                        one: low.one.clone(),
                    },
                    (true, false) => FixedBits {
                        empty: false,
                        zero: high.zero.clone(),
                        one: high.one.clone(),
                    },
                    (false, false) => FixedBits {
                        empty: false,
                        zero: low.zero.intersection(&high.zero),
                        one: low.one.intersection(&high.one),
                    },
                };
                if !fixed.empty {
                    if high.empty {
                        fixed.zero.set_bit(*wire as u64);
                    }
                    if low.empty {
                        fixed.one.set_bit(*wire as u64);
                    }
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
        self.reclaim_pending(alloc::vec![id]);
    }
    fn reclaim_pending(&mut self, mut pending: Vec<usize>) -> Vec<usize> {
        while let Some(id) = pending.pop() {
            if self.references[id] != 0 {
                continue;
            }
            let Some(node) = self.nodes[id].take() else {
                continue;
            };
            self.unique.remove(&node);
            // Empty support carries no reference-counted mask storage.
            self.fixed[id] = self.empty_fixed.clone();
            self.free.push(id);
            if let Node::Branch { low, high, .. } = node {
                for child in [low, high] {
                    self.references[child] -= 1;
                    if self.references[child] == 0 {
                        pending.push(child);
                    }
                }
            }
        }
        pending
    }
    pub fn zero(&mut self) -> usize {
        self.intern(Node::Leaf(BigInt::zero(), BigInt::zero()))
    }
    fn branch(&mut self, wire: usize, low: usize, high: usize) -> usize {
        assert!(wire < self.cells, "split/fuse wire exceeds its register");
        assert!(self.top(low).is_none_or(|child| child > wire)
            && self.top(high).is_none_or(|child| child > wire),
            "split/fuse child escaped the parent wire ordering");
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
            memo: &mut OperationCache<usize, usize>,
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
        visit(self, id, scalar, format, &mut OperationCache::default())
    }

    /// Apply one normalized Hadamard to a wire of the shared register. The
    /// decision diagram remains one correlated amplitude function; a skipped
    /// wire is treated as the equal low/high pair it denotes.
    pub fn hadamard(&mut self, root: usize, wire: usize, format: &FixedPointFormat) -> Result<usize, String> {
        if wire >= self.cells { return Err("Hadamard wire exceeds decision register".into()); }
        let scale = format.scale().to_biguint().ok_or("Hadamard scale is not positive")?;
        let inverse_sqrt_two = integer_sqrt(&(&scale * &scale / 2u8));
        let scalar = FixedComplex { re: BigInt::from(inverse_sqrt_two), im: BigInt::zero() };
        fn visit(a: &mut DecisionArena, id: usize, wire: usize, scalar: &FixedComplex,
            format: &FixedPointFormat, memo: &mut OperationCache<usize, usize>) -> usize {
            if let Some(&result) = memo.get(&id) { return result; }
            let result = match a.top(id) {
                Some(top) if top < wire => {
                    let (low, high) = a.split(id, top);
                    let low = visit(a, low, wire, scalar, format, memo);
                    let high = visit(a, high, wire, scalar, format, memo);
                    a.branch(top, low, high)
                }
                _ => {
                    let (low, high) = a.split(id, wire);
                    let plus = a.sum(low, high);
                    let negative_high = a.scale(high, &FixedComplex { re: -format.scale(), im: BigInt::zero() }, format);
                    let minus = a.sum(low, negative_high);
                    let plus = a.scale(plus, scalar, format);
                    let minus = a.scale(minus, scalar, format);
                    a.branch(wire, plus, minus)
                }
            };
            memo.insert(id, result);
            result
        }
        Ok(visit(self, root, wire, &scalar, format, &mut OperationCache::default()))
    }

    /// Apply a diagonal winding only on the conjunction of the supplied
    /// register literals. This keeps Fourier alignment on the same resident
    /// powerset diagram instead of opening per-digit measurement branches.
    pub fn controlled_winding(&mut self, root: usize, controls: &[(usize, bool)],
        numerator: &BigInt, denominator: &BigUint, format: &FixedPointFormat) -> Result<usize, String> {
        if controls.iter().any(|&(wire, _)| wire >= self.cells)
            || controls.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
            return Err("controlled winding requires ordered in-range wires".into());
        }
        let phase = FixedComplex::winding_twiddle(numerator, denominator, format)?;
        let zero = self.zero();
        let selected = self.conditional_literals(zero, root, controls);
        let rotated = self.scale(selected, &phase, format);
        Ok(self.conditional_literals(root, rotated, controls))
    }

    /// Apply the full binary Fourier transform to an ordered subset of wires.
    /// Controlled rotations remain diagonal on the shared decision graph; no
    /// phase digit is measured or fed back while the transform is running.
    pub fn fourier_register(&mut self, mut root: usize, wires: &[usize], inverse: bool,
        format: &FixedPointFormat) -> Result<usize, String> {
        if wires.iter().any(|&wire| wire >= self.cells)
            || wires.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err("Fourier register wires must be ordered and in range".into());
        }
        let swap = |arena: &mut DecisionArena, state: usize, left: usize, right: usize| {
            let state = arena.controlled_flip(state, right, &[left]);
            let state = arena.controlled_flip(state, left, &[right]);
            arena.controlled_flip(state, right, &[left])
        };
        if inverse {
            for index in 0..wires.len() / 2 {
                root = swap(self, root, wires[index], wires[wires.len() - 1 - index]);
            }
            for target_index in 0..wires.len() {
                let target = wires[target_index];
                for control_index in 0..target_index {
                    let control = wires[control_index];
                    let denominator = BigUint::one() << (target_index - control_index + 1);
                    root = self.controlled_winding(root, &[(control, true), (target, true)],
                        &-BigInt::one(), &denominator, format)?;
                }
                root = self.hadamard(root, target, format)?;
            }
        } else {
            for target_index in (0..wires.len()).rev() {
                let target = wires[target_index];
                root = self.hadamard(root, target, format)?;
                for control_index in (0..target_index).rev() {
                    let control = wires[control_index];
                    let denominator = BigUint::one() << (target_index - control_index + 1);
                    root = self.controlled_winding(root, &[(control, true), (target, true)],
                        &BigInt::one(), &denominator, format)?;
                }
            }
            for index in 0..wires.len() / 2 {
                root = swap(self, root, wires[index], wires[wires.len() - 1 - index]);
            }
        }
        Ok(root)
    }

    /// Sample the complete low-wire phase word with one Born quantile. Wires
    /// above `width` remain unobserved and are summed into each prefix mass.
    pub fn sample_prefix(&self, root: usize, width: usize, numerator: &BigUint,
        denominator: &BigUint) -> Result<BigUint, String> {
        if width > self.cells || denominator.is_zero() || numerator >= denominator {
            return Err("whole-register readout has an invalid width or Born quantile".into());
        }
        let mut masses = OperationCache::default();
        let total = self.mass(root);
        if total.is_zero() { return Err("whole-register readout has zero Born mass".into()); }
        let mut rank = &total * numerator / denominator;
        let mut current = root;
        let mut value = BigUint::zero();
        for wire in 0..width {
            let (low, high) = self.split(current, wire);
            let low_mass = self.mass_at(low, wire + 1, &mut masses);
            if rank < low_mass {
                current = low;
            } else {
                rank -= low_mass;
                value.set_bit(wire as u64, true);
                current = high;
            }
        }
        Ok(value)
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
            memo: &mut OperationCache<(usize, usize), usize>,
        ) -> usize {
            // Additive identity closes at every nested boundary, not only at
            // the outer sum. Preserve the other shared arm without walking it.
            if arena.fixed[a].empty { return b; }
            if arena.fixed[b].empty { return a; }
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
        visit(self, a, b, &mut OperationCache::default())
    }
    pub fn flip(&mut self, id: usize, target: usize) -> usize {
        fn visit(
            a: &mut DecisionArena,
            id: usize,
            t: usize,
            memo: &mut OperationCache<usize, usize>,
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
        visit(self, id, target, &mut OperationCache::default())
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
            memo: &mut OperationCache<(usize, usize), usize>,
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
        visit(self, root, target, &controls, 0, &mut OperationCache::default())
    }
    pub fn conditional(&mut self, a: usize, b: usize, controls: &[usize]) -> usize {
        fn visit(
            arena: &mut DecisionArena,
            a: usize,
            b: usize,
            controls: &[usize],
            at: usize,
            memo: &mut OperationCache<(usize, usize, usize), usize>,
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
        visit(self, a, b, controls, 0, &mut OperationCache::default())
    }

    pub fn controls_possible(&self, root: usize, controls: &[(usize, bool)]) -> bool {
        let fixed = &self.fixed[root];
        !fixed.empty && !controls.iter().any(|&(wire, positive)| {
            if positive { fixed.zero.bit(wire as u64) } else { fixed.one.bit(wire as u64) }
        })
    }
    /// Exact clean-work boundary predicate, using the same support metadata
    /// maintained by every branch constructor. Empty arms are vacuously clean.
    pub fn wire_is_zero(&self, root: usize, wire: usize) -> bool {
        self.fixed[root].empty || self.fixed[root].zero.bit(wire as u64)
    }

    /// Trace the shared work coordinates once per paired decision branch.
    /// Entry i,j is <work_i|work_j>; skipped wires retain their multiplicity.
    pub fn control_gram(&self, roots: &[usize;4]) -> [(BigInt,BigInt);16] {
        fn visit(a: &DecisionArena, left: usize, right: usize, depth: usize,
            memo: &mut OperationCache<(usize,usize,usize),(BigInt,BigInt)>) -> (BigInt,BigInt) {
            if a.fixed[left].empty || a.fixed[right].empty {
                return (BigInt::zero(),BigInt::zero());
            }
            if let Some(value) = memo.get(&(left,right,depth)) { return value.clone(); }
            let value = match (a.top(left),a.top(right)) {
                (None,None) => {
                    let (Node::Leaf(lr,li),Node::Leaf(rr,ri)) = (a.node(left),a.node(right)) else { unreachable!() };
                    ((lr*rr+li*ri) << (a.cells-depth),(lr*ri-li*rr) << (a.cells-depth))
                }
                (x,y) => {
                    let wire = x.into_iter().chain(y).min().unwrap();
                    let (ll,lh) = a.split(left,wire);
                    let (rl,rh) = a.split(right,wire);
                    let low = visit(a,ll,rl,wire+1,memo);
                    let high = visit(a,lh,rh,wire+1,memo);
                    ((low.0+high.0) << (wire-depth),(low.1+high.1) << (wire-depth))
                }
            };
            memo.insert((left,right,depth),value.clone());
            value
        }
        let mut memo = OperationCache::default();
        core::array::from_fn(|index| visit(self,roots[index/4],roots[index%4],0,&mut memo))
    }

    /// Fuse changed and unchanged arms on the original literal controls.
    pub fn conditional_literals(&mut self, a: usize, b: usize, controls: &[(usize, bool)]) -> usize {
        fn visit(arena: &mut DecisionArena, a: usize, b: usize,
            controls: &[(usize, bool)], at: usize,
            memo: &mut OperationCache<(usize, usize, usize), usize>) -> usize {
            if a == b { return a; }
            if at == controls.len() { return b; }
            if let Some(&result) = memo.get(&(a,b,at)) { return result; }
            // A fixed literal has already closed inside both nested arms.
            // Reuse that closure instead of walking every remaining wire to
            // reimpose the same selector. Empty support satisfies it vacuously.
            let (control,positive) = controls[at];
            let fixed_literal = |root:usize, value:bool| {
                let fixed = &arena.fixed[root];
                fixed.empty || if value { fixed.one.bit(control as u64) }
                    else { fixed.zero.bit(control as u64) }
            };
            if fixed_literal(a,positive) && fixed_literal(b,positive) {
                let result = visit(arena,a,b,controls,at+1,memo);
                memo.insert((a,b,at),result);
                return result;
            }
            if fixed_literal(a,!positive) && fixed_literal(b,!positive) {
                return a;
            }
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
        visit(self,a,b,controls,0,&mut OperationCache::default())
    }

    /// Reversible constant addition as a shared dyadic split/fuse transducer.
    /// Output coordinates select the original input by the subtraction borrow.
    /// The final borrow is discarded only at the register's modular boundary.
    pub fn add_constant(&mut self, root: usize, register: &[usize], value: &BigUint,
        controls: &[(usize, bool)]) -> usize {
        if !self.controls_possible(root,controls) || value.is_zero() { return root; }
        fn visit(arena: &mut DecisionArena, id: usize, register: &[usize], value: &BigUint,
            at: usize, borrow: bool, memo: &mut OperationCache<(usize,usize,bool),usize>) -> usize {
            // Subtraction by a zero suffix with no incoming borrow is the
            // identity on every remaining register and spectator wire.
            if !borrow && value.bits() <= at as u64 { return id; }
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
        let changed = visit(self,enabled,register,value,0,false,&mut OperationCache::default());
        self.conditional_literals(root,changed,controls)
    }

    fn partition_less(&mut self, root: usize, register: &[usize], value: &BigUint) -> (usize,usize) {
        let zero = self.zero();
        let fixed = &self.fixed[root];
        if fixed.empty { return (zero,zero); }
        if value.bits() > register.len() as u64 { return (root,zero); }
        // The exact support masks bound every live register value. Compare
        // those bounds from the most significant wire without constructing
        // either integer or walking the amplitude diagram. An uncertain
        // interval falls through to the full partition below.
        if let Some(less) = self.partition_suffix_bound(root,register,value,0,false) {
            return if less { (root,zero) } else { (zero,root) };
        }
        fn visit(arena: &mut DecisionArena, id: usize, register: &[usize], value: &BigUint,
            at: usize, less: bool, zero: usize,
            memo: &mut OperationCache<(usize,usize,bool),(usize,usize)>) -> (usize,usize) {
            if arena.fixed[id].empty { return (zero,zero); }
            if at == register.len() { return if less { (id,zero) } else { (zero,id) }; }
            if let Some(&result) = memo.get(&(id,at,less)) { return result; }
            if let Some(all_less) = arena.partition_suffix_bound(id,register,value,at,less) {
                let result = if all_less { (id,zero) } else { (zero,id) };
                memo.insert((id,at,less),result);
                return result;
            }
            let wire = arena.top(id).unwrap_or(register[at]).min(register[at]);
            let (low,high) = arena.split(id,wire);
            let (low,high) = if wire == register[at] {
                let bit = value.bit(at as u64);
                (visit(arena,low,register,value,at+1,bit || less,zero,memo),
                 visit(arena,high,register,value,at+1,bit && less,zero,memo))
            } else {
                (visit(arena,low,register,value,at,less,zero,memo),
                 visit(arena,high,register,value,at,less,zero,memo))
            };
            let result = (arena.branch(wire,low.0,high.0),arena.branch(wire,low.1,high.1));
            memo.insert((id,at,less),result);
            result
        }
        visit(self,root,register,value,0,false,zero,&mut OperationCache::default())
    }

    // Compare the remaining support interval with the remaining threshold.
    // Equal suffixes inherit the subtraction borrow from the consumed prefix.
    fn partition_suffix_bound(&self, root: usize, register: &[usize], value: &BigUint,
        at: usize, less: bool) -> Option<bool> {
        let fixed = &self.fixed[root];
        let mut minimum = core::cmp::Ordering::Equal;
        let mut maximum = core::cmp::Ordering::Equal;
        for (bit,&wire) in register.iter().enumerate().skip(at).rev() {
            let boundary = value.bit(bit as u64);
            if minimum == core::cmp::Ordering::Equal {
                minimum = fixed.one.bit(wire as u64).cmp(&boundary);
            }
            if maximum == core::cmp::Ordering::Equal {
                maximum = (!fixed.zero.bit(wire as u64)).cmp(&boundary);
            }
            if minimum != core::cmp::Ordering::Equal && maximum != core::cmp::Ordering::Equal {
                break;
            }
        }
        if maximum == core::cmp::Ordering::Less || (maximum == core::cmp::Ordering::Equal && less) {
            return Some(true);
        }
        if minimum == core::cmp::Ordering::Greater || (minimum == core::cmp::Ordering::Equal && !less) {
            return Some(false);
        }
        None
    }

    /// Remove one unchanged source coordinate in a shared traversal.
    /// Both cofactors retain every earlier work coordinate. The caller restores
    /// the source wire after translating the work register.
    fn source_cofactors(&mut self, root: usize, source: usize,
        memo: &mut OperationCache<(usize,usize),(usize,usize)>) -> (usize,usize) {
        fn visit(arena: &mut DecisionArena, root: usize, source: usize,
            memo: &mut OperationCache<(usize,usize),(usize,usize)>) -> (usize,usize) {
            if let Some(&result) = memo.get(&(root,source)) { return result; }
            let result = match *arena.node(root) {
                Node::Branch { wire, low, high } if wire == source => (low,high),
                Node::Branch { wire, low, high } if wire < source => {
                    let low = visit(arena,low,source,memo);
                    let high = visit(arena,high,source,memo);
                    (arena.branch(wire,low.0,high.0),arena.branch(wire,low.1,high.1))
                }
                _ => (root,root),
            };
            memo.insert((root,source),result);
            result
        }
        visit(self,root,source,memo)
    }

    /// Split only live source-digit arms, translate each complete digit once,
    /// and fuse their disjoint source coordinates without a candidate table.
    fn fuse_source_arms(&mut self, low: usize, high: usize, wire: usize) -> usize {
        let fused = self.conditional_literals(low, high, &[(wire, true)]);
        let recovered = self.source_cofactors(fused, wire, &mut OperationCache::default());
        debug_assert_eq!(recovered, (low, high),
            "transformed source arms did not recover at wire {wire}");
        fused
    }

    /// The two transformed intervals retain their exact complex amplitudes.
    /// Recover both arms through the output cut before the parent can return.
    fn fuse_interval_arms(&mut self, low: usize, high: usize,
        register: &[usize], cut: &BigUint) -> usize {
        let fused = self.sum(low, high);
        let recovered = self.partition_less(fused, register, cut);
        debug_assert_eq!(recovered, (low, high),
            "transformed interval arms did not recover at their output cut");
        fused
    }

    pub fn modular_digit_add(&mut self, root: usize, register: &[usize], digit: &[usize],
        value: &BigUint, modulus: &BigUint, controls: &[(usize,bool)]) -> usize {
        if !self.controls_possible(root,controls) { return root; }
        fn visit_precomputed(arena: &mut DecisionArena, root: usize, register: &[usize], digit: &[usize],
            at: usize, mask: usize, sums: &[BigUint], modulus: &BigUint,
            cofactors: &mut OperationCache<(usize,usize),(usize,usize)>) -> usize {
            if arena.fixed[root].empty { return root; }
            if at == digit.len() { return arena.modular_add(root,register,&sums[mask],modulus,&[]); }
            let bit = digit[at];
            if arena.fixed[root].zero.bit(bit as u64) {
                return visit_precomputed(arena,root,register,digit,at+1,mask,sums,modulus,cofactors);
            }
            let high_mask = mask | (1usize << at);
            if arena.fixed[root].one.bit(bit as u64) {
                return visit_precomputed(arena,root,register,digit,at+1,high_mask,sums,modulus,cofactors);
            }
            let (low,high) = arena.source_cofactors(root,bit,cofactors);
            let low = visit_precomputed(arena,low,register,digit,at+1,mask,sums,modulus,cofactors);
            let high = visit_precomputed(arena,high,register,digit,at+1,high_mask,sums,modulus,cofactors);
            arena.fuse_source_arms(low,high,bit)
        }
        fn visit(arena: &mut DecisionArena, root: usize, register: &[usize], digit: &[usize],
            at: usize, accumulated: &BigUint, place: &BigUint, modulus: &BigUint,
            cofactors: &mut OperationCache<(usize,usize),(usize,usize)>) -> usize {
            if arena.fixed[root].empty { return root; }
            if at == digit.len() { return arena.modular_add(root,register,accumulated,modulus,&[]); }
            let next_place = (place << 1usize) % modulus;
            if arena.fixed[root].zero.bit(digit[at] as u64) {
                return visit(arena,root,register,digit,at+1,accumulated,&next_place,modulus,cofactors);
            }
            let next_accumulated = (accumulated + place) % modulus;
            if arena.fixed[root].one.bit(digit[at] as u64) {
                return visit(arena,root,register,digit,at+1,&next_accumulated,&next_place,modulus,cofactors);
            }
            let (low,high) = arena.source_cofactors(root,digit[at],cofactors);
            let low = visit(arena,low,register,digit,at+1,accumulated,&next_place,modulus,cofactors);
            let high = visit(arena,high,register,digit,at+1,&next_accumulated,&next_place,modulus,cofactors);
            // Source and work coordinates are disjoint. Restore the removed
            // source wire in diagram order, including earlier work branches
            // changed by the translation, rather than adding masked roots.
            arena.fuse_source_arms(low, high, digit[at])
        }
        let zero = self.zero();
        let enabled = self.conditional_literals(zero,root,controls);
        // Nested digit arms share cofactor answers. Nodes are immutable and
        // none are reclaimed until this arithmetic operation has returned.
        let mut cofactors = OperationCache::default();
        let changed = if digit.len() <= 8 {
            // Every branch at depth `at` reaches the same at-most-2^8 prefix
            // sums. Compute them once instead of doing BigUint add/division at
            // every diagram node in the recursive traversal.
            let mut places = Vec::with_capacity(digit.len());
            let mut place = value % modulus;
            for _ in digit {
                places.push(place.clone());
                place = (&place << 1usize) % modulus;
            }
            let mut sums = alloc::vec![BigUint::zero(); 1usize << digit.len()];
            for mask in 1..sums.len() {
                let bit = mask.trailing_zeros() as usize;
                let previous = mask & (mask - 1);
                sums[mask] = (&sums[previous] + &places[bit]) % modulus;
            }
            visit_precomputed(self,enabled,register,digit,0,0,&sums,modulus,&mut cofactors)
        } else {
            visit(self,enabled,register,digit,0,&BigUint::zero(),value,modulus,&mut cofactors)
        };
        self.conditional_literals(root,changed,controls)
    }

    /// Fuse the two translated intervals of a complete modular addition.
    /// Invalid register values and disabled control arms stay unchanged.
    /// No borrow flag is materialized between the input and output boundary.
    pub fn modular_add(&mut self, root: usize, register: &[usize], value: &BigUint,
        modulus: &BigUint, controls: &[(usize,bool)]) -> usize {
        if !self.controls_possible(root,controls) { return root; }
        let value = value % modulus;
        if value.is_zero() { return root; }
        VOX_MODULAR_COUNTERS[0].fetch_add(1,Ordering::Relaxed);
        VOX_MODULAR_COUNTERS[1].store(1,Ordering::Relaxed);
        VOX_MODULAR_COUNTERS[2].store(self.retained_nodes() as u64,Ordering::Relaxed);
        let zero = self.zero();
        let enabled = self.conditional_literals(zero,root,controls);
        let (valid,invalid) = self.partition_less(enabled,register,modulus);
        let threshold = modulus - &value;
        let (low,high) = self.partition_less(valid,register,&threshold);
        VOX_MODULAR_COUNTERS[1].store(2,Ordering::Relaxed);
        let low = self.add_constant(low,register,&value,&[]);
        VOX_MODULAR_COUNTERS[1].store(3,Ordering::Relaxed);
        let radix = BigUint::from(1u8) << register.len();
        let high = self.add_constant(high,register,&(radix-threshold),&[]);
        VOX_MODULAR_COUNTERS[1].store(4,Ordering::Relaxed);
        // The wrapped interval lands below value; the unwrapped interval
        // lands above it. Recovery therefore uses the transformed cut.
        let changed = self.fuse_interval_arms(high, low, register, &value);
        VOX_MODULAR_COUNTERS[1].store(5,Ordering::Relaxed);
        let changed = self.fuse_interval_arms(changed, invalid, register, modulus);
        let result = self.conditional_literals(root,changed,controls);
        VOX_MODULAR_COUNTERS[3].store(self.retained_nodes() as u64,Ordering::Relaxed);
        VOX_MODULAR_COUNTERS[1].store(0,Ordering::Relaxed);
        result
    }

    /// XOR a high flag with the exact less-than predicate on a register.
    /// Each more significant bit updates the comparison carried by its inner
    /// dyad; the flag is transformed after the complete register has fused.
    pub fn compare_constant(&mut self, root: usize, register: &[usize], value: &BigUint,
        controls: &[(usize,bool)], flag: usize) -> usize {
        if !self.controls_possible(root,controls) || value.is_zero() { return root; }
        fn visit(arena: &mut DecisionArena,id: usize,register: &[usize],value: &BigUint,
            at: usize,less: bool,flag: usize,memo: &mut OperationCache<(usize,usize,bool),usize>) -> usize {
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
        let changed = visit(self,enabled,register,value,0,false,flag,&mut OperationCache::default());
        self.conditional_literals(root,changed,controls)
    }
    fn mass_at(&self, root: usize, depth: usize,
        memo: &mut OperationCache<(usize, usize), BigUint>) -> BigUint {
        if let Some(mass) = memo.get(&(root, depth)) { return mass.clone(); }
        let mass = match self.node(root) {
            Node::Leaf(re, im) => (re * re + im * im).to_biguint().unwrap() << (self.cells - depth),
            &Node::Branch { wire, low, high } =>
                (self.mass_at(low, wire + 1, memo) + self.mass_at(high, wire + 1, memo)) << (wire - depth),
        };
        memo.insert((root, depth), mass.clone());
        mass
    }
    pub fn mass(&self, root: usize) -> BigUint {
        self.mass_at(root, 0, &mut OperationCache::default())
    }
    pub fn normalize(&mut self, root: usize, norm: &BigInt, scale: &BigInt) -> usize {
        fn visit(
            a: &mut DecisionArena,
            id: usize,
            norm: &BigInt,
            scale: &BigInt,
            memo: &mut OperationCache<usize, usize>,
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
        visit(self, root, norm, scale, &mut OperationCache::default())
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
        // Drain the operation's existing allocation list as one work stack.
        // Reclaimed children and already released slots are checked in the
        // same pass, without allocating a fresh stack for each temporary root.
        self.created = self.reclaim_pending(created);
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
