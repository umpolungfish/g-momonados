//! Shared complex-amplitude decision branches for a coherent work register.
//! Equal branches share storage; gates act on the diagram without enumerating residues.
use crate::phase_unbraid::{FixedComplex, FixedPointFormat};
use crate::word_tape::{SignedWordTape, WordTape};
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
    Leaf(SignedWordTape, SignedWordTape),
    Branch {
        wire: usize,
        low: usize,
        high: usize,
    },
}

impl Node {
    // Compatibility boundary for prepared coefficients and existing observers.
    // Resident leaves and all leaf arithmetic retain the IMASM word rails.
    fn leaf(re: BigInt, im: BigInt) -> Self {
        Self::Leaf(SignedWordTape::from_bigint(&re), SignedWordTape::from_bigint(&im))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::One;

    #[cfg(feature="hosted")]
    #[test]
    fn compiled_support_preserves_locked_source_register_and_reversible_recovery() {
        use crate::baked_resident::{BakedNode,BakedSupport,BakedSignedWord,BakedWordComplex};
        let source=WordTape::from_canonical_word(include_str!(
            "../measurements/resident_imasm_integration_20261012/inputs/fresh_balanced_512.imasm").trim()).unwrap();
        assert!(source.bit_len()>=200);
        let mut original=DecisionArena::new(source.bit_len());
        let base=source.to_biguint();
        let a=original.basis(&base,BigInt::from(3));
        let b=original.basis(&(&base^BigUint::one()),BigInt::from(-5));
        let root=original.sum(a,b);
        let record=original.prepared_record(root);
        let signed=|row:&serde_json::Value| {
            let value=SignedWordTape::from_prepared_record(row).unwrap();
            BakedSignedWord {negative:value.is_negative(),
                word:Box::leak(value.magnitude().as_word().to_owned().into_boxed_str())}
        };
        let rows:Vec<_>=record["nodes"].as_array().unwrap().iter().map(|row| {
            if row.get("re").is_some() {BakedNode::Leaf(BakedWordComplex {re:signed(&row["re"]),im:signed(&row["im"])})}
            else {BakedNode::Branch {wire:row["wire"].as_u64().unwrap() as usize,
                low:row["low"].as_u64().unwrap() as usize,high:row["high"].as_u64().unwrap() as usize}}
        }).collect();
        let mut limbs=Vec::new();
        let mut spans=Vec::new();
        for row in record["support_masks"].as_array().unwrap() {
            let digits:Vec<u32>=row.as_array().unwrap().iter().map(|n|n.as_u64().unwrap() as u32).collect();
            let start=limbs.len();limbs.extend(digits);spans.push((start,limbs.len()));
        }
        let fixed:Vec<_>=record["fixed"].as_array().unwrap().iter().map(|row|
            (row[0].as_bool().unwrap(),row[1].as_u64().unwrap() as usize,row[2].as_u64().unwrap() as usize)).collect();
        let support=BakedSupport {limbs:Box::leak(limbs.into_boxed_slice()),
            spans:Box::leak(spans.into_boxed_slice()),fixed:Box::leak(fixed.into_boxed_slice())};
        let (mut imported,imported_root)=DecisionArena::from_baked_nodes(source.bit_len(),
            record["root"].as_u64().unwrap() as usize,&rows,&support).unwrap();
        assert_eq!(imported.prepared_record(imported_root),record);
        let changed=imported.flip(imported_root,source.bit_len()-1);
        let recovered=imported.flip(changed,source.bit_len()-1);
        assert_eq!(recovered,imported_root);
        for value in [&base,&(&base^BigUint::one())] {
            assert_eq!(imported.amplitude(recovered,value),original.amplitude(root,value));
        }
        let mut corrupt=record.clone();
        let last=corrupt["fixed"].as_array().unwrap().len()-1;
        corrupt["fixed"][last][0]=serde_json::json!(true);
        assert!(DecisionArena::from_prepared_record(&corrupt).is_err());
    }

    #[test]
    fn imported_disjoint_support_preserves_every_signed_complex_amplitude() {
        let mut left = DecisionArena::new(2);
        let zero = left.zero();
        let mut l = zero;
        for value in 0u8..4 {
            let leaf = left.intern(Node::leaf(BigInt::from(value+1), -BigInt::from(value+3)));
            let mut branch = leaf;
            for wire in (0..2).rev() {
                branch = if value>>wire & 1 == 1 { left.branch(wire,zero,branch) }
                    else { left.branch(wire,branch,zero) };
            }
            l = left.sum(l, branch);
        }
        let mut right = DecisionArena::new(2);
        let r = right.basis(&BigUint::from(3u8), BigInt::from(-7));
        let mut joint = DecisionArena::new(4);
        let a = joint.import_shifted(&left,l,0).unwrap();
        let b = joint.import_shifted(&right,r,2).unwrap();
        let joined = joint.amplitude_product(a,b);
        assert_eq!(joint.mass(joined), left.mass(l)*right.mass(r));
        for first in 0u8..4 {
            for second in 0u8..4 {
                let (ar,ai) = left.amplitude(l,&BigUint::from(first));
                let (br,bi) = right.amplitude(r,&BigUint::from(second));
                assert_eq!(joint.amplitude(joined,&BigUint::from(first | second<<2)),
                    (&ar*&br-&ai*&bi, ar*bi+ai*br));
            }
        }
        assert!(joint.import_shifted(&left,l,3).is_err());
    }

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
    fn sic_matrix_operation_preserves_complex_coherence_and_work_ancestry() {
        let mut arena = DecisionArena::new(3);
        let first = arena.basis(&BigUint::zero(), BigInt::one());
        let imaginary = arena.basis(&BigUint::one(), BigInt::one());
        let imaginary = arena.scale_words(imaginary,
            &(SignedWordTape::zero(), SignedWordTape::from_small(1)), 0);
        let other_work = arena.basis(&BigUint::from(6u8), BigInt::from(2u8));
        let root = arena.sum(first, imaginary);
        let root = arena.sum(root, other_work);
        let zero = arena.zero();
        let mut aligned = [zero;4];
        for (label, arm) in aligned.iter_mut().enumerate() {
            *arm = arena.conditional_literals(zero,root,
                &[(0,label&1 != 0),(1,label&2 != 0)]);
            for bit in 0..2 { if label&(1<<bit) != 0 { *arm = arena.flip(*arm,bit); } }
        }
        let gram = arena.control_gram(&aligned);
        let frame = crate::anyon_ququart::FixedQuquartSic::new(
            &FixedPointFormat::for_modulus_bits(12).unwrap()).unwrap();
        let masses = frame.gram_mass_numerators(&gram).unwrap();
        let recovered = frame.certify_gram_frame_unrounded_masses(&gram,&masses).unwrap();
        let matrix = core::array::from_fn(|entry| {
            let (re,im) = &recovered.rounded.recovered[4*(entry%4)+entry/4];
            (SignedWordTape::from_bigint(re),SignedWordTape::from_bigint(im))
        });
        let result = arena.apply_control_matrix(root,[0,1],&matrix).unwrap();
        for address in 0u8..8 {
            let expected = match address {
                0 => (BigInt::from(2),BigInt::zero()),
                1 => (BigInt::zero(),BigInt::from(2)),
                6 => (BigInt::from(8),BigInt::zero()),
                _ => (BigInt::zero(),BigInt::zero()),
            };
            assert_eq!(arena.amplitude(result,&BigUint::from(address)), expected);
        }
        assert!(arena.apply_control_matrix(root,[1,0],&matrix).is_err());
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
                    let mut basis = arena.intern(Node::leaf(re.clone(),im.clone()));
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
                            let mut basis = arena.intern(Node::leaf(re.clone(),im.clone()));
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
            let mut imaginary = arena.intern(Node::leaf(BigInt::zero(),BigInt::from(2*value+7)));
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
                    let mut basis = arena.intern(Node::leaf(amplitude.0.clone(),amplitude.1.clone()));
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
        let amplitude = arena.intern(Node::leaf(BigInt::from(3), BigInt::from(5)));
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
        self.intern_fixed(node, fixed)
    }
    fn intern_fixed(&mut self, node: Node, fixed: FixedBits) -> usize {
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
        self.intern(Node::Leaf(SignedWordTape::zero(), SignedWordTape::zero()))
    }
    /// Import a retained amplitude function into a disjoint cell range.
    /// Unaddressed cells remain free; no basis branches are enumerated.
    pub fn import_shifted(&mut self, source: &Self, root: usize, offset: usize)
        -> Result<usize, String> {
        if offset.checked_add(source.cells).is_none_or(|end| end > self.cells) {
            return Err("imported decision register exceeds its carrier".into());
        }
        fn visit(target: &mut DecisionArena, source: &DecisionArena, root: usize,
            offset: usize, memo: &mut OperationCache<usize, usize>) -> usize {
            if let Some(&result) = memo.get(&root) { return result; }
            let result = match source.node(root) {
                Node::Leaf(re, im) => target.intern(Node::Leaf(re.clone(), im.clone())),
                Node::Branch { wire, low, high } => {
                    let low = visit(target, source, *low, offset, memo);
                    let high = visit(target, source, *high, offset, memo);
                    target.branch(wire+offset, low, high)
                }
            };
            memo.insert(root, result);
            result
        }
        Ok(visit(self, source, root, offset, &mut OperationCache::default()))
    }

    /// Multiply amplitude functions pointwise. On disjoint imported ranges
    /// this is their tensor product in one resident diagram.
    pub fn amplitude_product(&mut self, first: usize, second: usize) -> usize {
        fn visit(arena: &mut DecisionArena, first: usize, second: usize,
            memo: &mut OperationCache<(usize,usize), usize>) -> usize {
            if let Some(&result) = memo.get(&(first,second)) { return result; }
            let result = match (arena.node(first).clone(), arena.node(second).clone()) {
                (Node::Leaf(ar,ai), Node::Leaf(br,bi)) =>
                    arena.intern(Node::Leaf(ar.mul(&br).sub(&ai.mul(&bi)),
                        ar.mul(&bi).add(&ai.mul(&br)))),
                _ => {
                    let wire = match (arena.top(first), arena.top(second)) {
                        (Some(a),Some(b)) => a.min(b),
                        (Some(a),None) | (None,Some(a)) => a,
                        _ => unreachable!(),
                    };
                    let (al,ah) = arena.split(first,wire);
                    let (bl,bh) = arena.split(second,wire);
                    let low = visit(arena,al,bl,memo);
                    let high = visit(arena,ah,bh,memo);
                    arena.branch(wire,low,high)
                }
            };
            memo.insert((first,second),result);
            result
        }
        visit(self,first,second,&mut OperationCache::default())
    }

    /// Rename physical cells without changing any amplitude or free branch.
    /// Selectors restore canonical ordering when the permutation crosses wires.
    pub fn permuted(&self, root: usize, wires: &[usize]) -> Result<(Self, usize), String> {
        let mut sorted = wires.to_vec();
        sorted.sort_unstable();
        if sorted != (0..self.cells).collect::<Vec<_>>() {
            return Err("register permutation must retain every cell exactly once".into());
        }
        fn visit(source: &DecisionArena, target: &mut DecisionArena, id: usize,
            wires: &[usize], fixed: &[Option<bool>], memo: &mut OperationCache<usize, usize>) -> usize {
            if let Some(&result) = memo.get(&id) { return result; }
            let result = match source.node(id) {
                Node::Leaf(re, im) => target.intern(Node::Leaf(re.clone(), im.clone())),
                Node::Branch { wire, low, high } => {
                    if let Some(value) = fixed[*wire] {
                        visit(source,target,if value { *high } else { *low },wires,fixed,memo)
                    } else {
                        let low = visit(source, target, *low, wires, fixed, memo);
                        let high = visit(source, target, *high, wires, fixed, memo);
                        let wire = wires[*wire];
                        if target.top(low).is_none_or(|child| child>wire)
                            && target.top(high).is_none_or(|child| child>wire) {
                            target.branch(wire,low,high)
                        } else { target.conditional_literals(low, high, &[(wire, true)]) }
                    }
                }
            };
            memo.insert(id, result);
            result
        }
        let mut target = Self::new(self.cells);
        let support = &self.fixed[root];
        let fixed: Vec<_> = (0..self.cells).map(|wire|
            if support.empty { None }
            else if support.zero.bit(wire as u64) { Some(false) }
            else if support.one.bit(wire as u64) { Some(true) } else { None }).collect();
        let root = visit(self, &mut target, root, wires, &fixed, &mut OperationCache::default());
        let mut literals: Vec<_> = fixed.iter().enumerate().filter_map(|(wire,value)|
            value.map(|value| (wires[wire],value))).collect();
        literals.sort_unstable();
        let zero = target.zero();
        let root = target.conditional_literals(zero,root,&literals);
        Ok((target, root))
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
        let mut root = self.intern(Node::leaf(scale, BigInt::zero()));
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
        self.scale_words(id, &(SignedWordTape::from_bigint(&scalar.re),
            SignedWordTape::from_bigint(&scalar.im)), format.w_bits as usize)
    }

    pub(crate) fn scale_words(&mut self, id: usize,
        scalar: &(SignedWordTape, SignedWordTape), fractional_bits: usize) -> usize {
        if scalar.0.is_zero() && scalar.1.is_zero() {
            return self.zero();
        }
        fn visit(
            a: &mut DecisionArena,
            id: usize,
            scalar: &(SignedWordTape, SignedWordTape),
            fractional_bits: usize,
            memo: &mut OperationCache<usize, usize>,
        ) -> usize {
            if let Some(r) = memo.get(&id) {
                return *r;
            }
            let r = match a.node(id).clone() {
                Node::Leaf(re, im) => {
                    let real = re.mul(&scalar.0).sub(&im.mul(&scalar.1))
                        .rounded_shr(fractional_bits);
                    let imaginary = re.mul(&scalar.1).add(&im.mul(&scalar.0))
                        .rounded_shr(fractional_bits);
                    a.intern(Node::Leaf(real, imaginary))
                }
                Node::Branch { wire, low, high } => {
                    let low = visit(a, low, scalar, fractional_bits, memo);
                    let high = visit(a, high, scalar, fractional_bits, memo);
                    a.branch(wire, low, high)
                }
            };
            memo.insert(id, r);
            r
        }
        visit(self, id, scalar, fractional_bits, &mut OperationCache::default())
    }

    /// A complete complex coordinate operator on two cells of the resident
    /// register. Work branches stay attached while the four control arms are
    /// aligned, transformed and returned to their original cell addresses.
    pub(crate) fn apply_control_matrix(&mut self, root: usize, wires: [usize; 2],
        matrix: &[(SignedWordTape, SignedWordTape); 16]) -> Result<usize, String> {
        if wires[0] >= wires[1] || wires[1] >= self.cells {
            return Err("coordinate operator needs two ordered resident cells".into());
        }
        let zero = self.zero();
        let mut aligned = [zero; 4];
        for (label, arm) in aligned.iter_mut().enumerate() {
            let controls = [(wires[0], label & 1 != 0), (wires[1], label & 2 != 0)];
            *arm = self.conditional_literals(zero, root, &controls);
            for (wire, active) in controls {
                if active { *arm = self.flip(*arm, wire); }
            }
        }
        let mut result = zero;
        for row in 0..4 {
            let mut arm = zero;
            for col in 0..4 {
                let transformed = self.scale_words(aligned[col], &matrix[4*row+col], 0);
                arm = self.sum(arm, transformed);
            }
            for (bit, wire) in wires.into_iter().enumerate() {
                if row & (1 << bit) != 0 { arm = self.flip(arm, wire); }
            }
            result = self.sum(result, arm);
        }
        Ok(result)
    }

    /// Apply one normalized Hadamard to a wire of the shared register. The
    /// decision diagram remains one correlated amplitude function; a skipped
    /// wire is treated as the equal low/high pair it denotes.
    pub fn hadamard(&mut self, root: usize, wire: usize, format: &FixedPointFormat) -> Result<usize, String> {
        if wire >= self.cells { return Err("Hadamard wire exceeds decision register".into()); }
        let scale = WordTape::one_at(format.w_bits as usize);
        let inverse_sqrt_two = scale.mul(&scale).shr1().isqrt();
        let scalar = FixedComplex { re: BigInt::from(inverse_sqrt_two.to_biguint()), im: BigInt::zero() };
        self.hadamard_with_scalar(root,wire,format,&scalar)
    }

    pub(crate) fn hadamard_with_scalar(&mut self, root: usize, wire: usize,
        format: &FixedPointFormat, scalar: &FixedComplex) -> Result<usize,String> {
        if wire >= self.cells { return Err("Hadamard wire exceeds decision register".into()); }
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
                    let negative_high = a.scale_words(high,
                        &(SignedWordTape::from_magnitude(WordTape::one_at(format.w_bits as usize),true),
                            SignedWordTape::zero()),format.w_bits as usize);
                    let minus = a.sum(low, negative_high);
                    let plus = a.scale(plus, scalar, format);
                    let minus = a.scale(minus, scalar, format);
                    a.branch(wire, plus, minus)
                }
            };
            memo.insert(id, result);
            result
        }
        Ok(visit(self, root, wire, scalar, format, &mut OperationCache::default()))
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
        self.fourier_register_with_coefficients(&mut root,wires,inverse,format,None,None)
    }

    pub(crate) fn fourier_register_prepared(&mut self, mut root: usize, wires: &[usize],
        format: &FixedPointFormat, inverse_phases: &[FixedComplex], hadamard: &FixedComplex) -> Result<usize,String> {
        if inverse_phases.len()!=wires.len().saturating_sub(1) {
            return Err("prepared Fourier coefficients changed register width".into());
        }
        self.fourier_register_with_coefficients(&mut root,wires,true,format,Some(inverse_phases),Some(hadamard))
    }

    fn fourier_register_with_coefficients(&mut self, state: &mut usize, wires: &[usize], inverse: bool,
        format: &FixedPointFormat, phases: Option<&[FixedComplex]>, hadamard: Option<&FixedComplex>) -> Result<usize,String> {
        let mut root = *state;
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
                    let controls = [(control,true),(target,true)];
                    root = if let Some(phases) = phases {
                        let zero = self.zero();
                        let selected = self.conditional_literals(zero,root,&controls);
                        let rotated = self.scale(selected,&phases[target_index-control_index-1],format);
                        self.conditional_literals(root,rotated,&controls)
                    } else {
                        let denominator = WordTape::one_at(target_index-control_index+1).to_biguint();
                        self.controlled_winding(root,&controls,&BigInt::from(-1),&denominator,format)?
                    };
                }
                root = if let Some(scalar) = hadamard { self.hadamard_with_scalar(root,target,format,scalar)? }
                    else { self.hadamard(root,target,format)? };
            }
        } else {
            for target_index in (0..wires.len()).rev() {
                let target = wires[target_index];
                root = self.hadamard(root, target, format)?;
                for control_index in (0..target_index).rev() {
                    let control = wires[control_index];
                    let denominator = WordTape::one_at(target_index-control_index+1).to_biguint();
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
        self.sample_word(root, width, &WordTape::from_biguint(numerator),
            &WordTape::from_biguint(denominator)).map(|word| word.to_biguint())
    }
    pub fn sample_word(&self, root: usize, width: usize, numerator: &WordTape,
        denominator: &WordTape) -> Result<WordTape, String> {
        if width > self.cells || denominator.is_zero() || numerator.ge(denominator) {
            return Err("whole-register readout has an invalid width or Born quantile".into());
        }
        let mut masses = OperationCache::default();
        let total = self.mass_word(root);
        if total.is_zero() { return Err("whole-register readout has zero Born mass".into()); }
        let mut rank = total.mul(numerator).divmod(denominator).unwrap().0;
        let mut current = root;
        let mut value = WordTape::zero();
        for wire in 0..width {
            let (low, high) = self.split(current, wire);
            let low_mass = self.mass_at(low, wire + 1, &mut masses);
            if !rank.ge(&low_mass) {
                current = low;
            } else {
                rank = rank.sub(&low_mass).unwrap();
                value = value.add(&WordTape::one_at(wire));
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
                    arena.intern(Node::Leaf(ar.add(br), ai.add(bi)))
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
            memo: &mut OperationCache<(usize,usize,usize),(SignedWordTape,SignedWordTape)>) -> (SignedWordTape,SignedWordTape) {
            if a.fixed[left].empty || a.fixed[right].empty {
                return (SignedWordTape::zero(),SignedWordTape::zero());
            }
            if let Some(value) = memo.get(&(left,right,depth)) { return value.clone(); }
            let value = match (a.top(left),a.top(right)) {
                (None,None) => {
                    let (Node::Leaf(lr,li),Node::Leaf(rr,ri)) = (a.node(left),a.node(right)) else { unreachable!() };
                    (lr.mul(rr).add(&li.mul(ri)).shl(a.cells-depth),
                        lr.mul(ri).sub(&li.mul(rr)).shl(a.cells-depth))
                }
                (x,y) => {
                    let wire = x.into_iter().chain(y).min().unwrap();
                    let (ll,lh) = a.split(left,wire);
                    let (rl,rh) = a.split(right,wire);
                    let low = visit(a,ll,rl,wire+1,memo);
                    let high = visit(a,lh,rh,wire+1,memo);
                    (low.0.add(&high.0).shl(wire-depth),low.1.add(&high.1).shl(wire-depth))
                }
            };
            memo.insert((left,right,depth),value.clone());
            value
        }
        let mut memo = OperationCache::default();
        core::array::from_fn(|index| {
            let (re, im) = visit(self,roots[index/4],roots[index%4],0,&mut memo);
            (re.to_bigint(), im.to_bigint())
        })
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
        self.add_constant_word(root, register, &WordTape::from_biguint(value), controls)
    }
    fn add_constant_word(&mut self, root: usize, register: &[usize], value: &WordTape,
        controls: &[(usize, bool)]) -> usize {
        if !self.controls_possible(root,controls) || value.is_zero() { return root; }
        fn visit(arena: &mut DecisionArena, id: usize, register: &[usize], value: &WordTape,
            at: usize, borrow: bool, memo: &mut OperationCache<(usize,usize,bool),usize>) -> usize {
            // Subtraction by a zero suffix with no incoming borrow is the
            // identity on every remaining register and spectator wire.
            if !borrow && value.bit_len() <= at { return id; }
            if at == register.len() || matches!(arena.node(id),Node::Leaf(..)) { return id; }
            if let Some(&result) = memo.get(&(id,at,borrow)) { return result; }
            let wire = arena.top(id).unwrap().min(register[at]);
            let (low,high) = arena.split(id,wire);
            let (low,high) = if wire == register[at] {
                let constant = value.bit_at(at);
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
        self.partition_less_word(root, register, &WordTape::from_biguint(value))
    }
    fn partition_less_word(&mut self, root: usize, register: &[usize], value: &WordTape) -> (usize,usize) {
        let zero = self.zero();
        let fixed = &self.fixed[root];
        if fixed.empty { return (zero,zero); }
        if value.bit_len() > register.len() { return (root,zero); }
        // The exact support masks bound every live register value. Compare
        // those bounds from the most significant wire without constructing
        // either integer or walking the amplitude diagram. An uncertain
        // interval falls through to the full partition below.
        if let Some(less) = self.partition_suffix_bound(root,register,value,0,false) {
            return if less { (root,zero) } else { (zero,root) };
        }
        fn visit(arena: &mut DecisionArena, id: usize, register: &[usize], value: &WordTape,
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
                let bit = value.bit_at(at);
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
    fn partition_suffix_bound(&self, root: usize, register: &[usize], value: &WordTape,
        at: usize, less: bool) -> Option<bool> {
        let fixed = &self.fixed[root];
        let mut minimum = core::cmp::Ordering::Equal;
        let mut maximum = core::cmp::Ordering::Equal;
        for (bit,&wire) in register.iter().enumerate().skip(at).rev() {
            let boundary = value.bit_at(bit);
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
        self.fuse_interval_word(low, high, register, &WordTape::from_biguint(cut))
    }
    fn fuse_interval_word(&mut self, low: usize, high: usize,
        register: &[usize], cut: &WordTape) -> usize {
        let fused = self.sum(low, high);
        let recovered = self.partition_less_word(fused, register, cut);
        debug_assert_eq!(recovered, (low, high),
            "transformed interval arms did not recover at their output cut");
        fused
    }

    pub fn modular_digit_add(&mut self, root: usize, register: &[usize], digit: &[usize],
        value: &BigUint, modulus: &BigUint, controls: &[(usize,bool)]) -> usize {
        if !self.controls_possible(root,controls) { return root; }
        let modulus = WordTape::from_biguint(modulus);
        let value = WordTape::from_biguint(value).divmod(&modulus)
            .expect("modular word addition requires a nonzero modulus").1;
        fn visit_precomputed(arena: &mut DecisionArena, root: usize, register: &[usize], digit: &[usize],
            at: usize, mask: usize, sums: &[WordTape], modulus: &WordTape,
            cofactors: &mut OperationCache<(usize,usize),(usize,usize)>) -> usize {
            if arena.fixed[root].empty { return root; }
            if at == digit.len() { return arena.modular_add_word(root,register,&sums[mask],modulus,&[]); }
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
            at: usize, accumulated: &WordTape, place: &WordTape, modulus: &WordTape,
            cofactors: &mut OperationCache<(usize,usize),(usize,usize)>) -> usize {
            if arena.fixed[root].empty { return root; }
            if at == digit.len() { return arena.modular_add_word(root,register,accumulated,modulus,&[]); }
            let next_place = place.add_mod_canonical(place, modulus);
            if arena.fixed[root].zero.bit(digit[at] as u64) {
                return visit(arena,root,register,digit,at+1,accumulated,&next_place,modulus,cofactors);
            }
            let next_accumulated = accumulated.add_mod_canonical(place, modulus);
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
            // sums. Retain those sums as words for every diagram return.
            let mut places = Vec::with_capacity(digit.len());
            let mut place = value.clone();
            for _ in digit {
                places.push(place.clone());
                place = place.add_mod_canonical(&place, &modulus);
            }
            let mut sums = alloc::vec![WordTape::zero(); 1usize << digit.len()];
            for mask in 1..sums.len() {
                let bit = mask.trailing_zeros() as usize;
                let previous = mask & (mask - 1);
                sums[mask] = sums[previous].add_mod_canonical(&places[bit], &modulus);
            }
            visit_precomputed(self,enabled,register,digit,0,0,&sums,&modulus,&mut cofactors)
        } else {
            visit(self,enabled,register,digit,0,&WordTape::zero(),&value,&modulus,&mut cofactors)
        };
        self.conditional_literals(root,changed,controls)
    }

    /// Fuse the two translated intervals of a complete modular addition.
    /// Invalid register values and disabled control arms stay unchanged.
    /// No borrow flag is materialized between the input and output boundary.
    pub fn modular_add(&mut self, root: usize, register: &[usize], value: &BigUint,
        modulus: &BigUint, controls: &[(usize,bool)]) -> usize {
        self.modular_add_word(root, register, &WordTape::from_biguint(value),
            &WordTape::from_biguint(modulus), controls)
    }
    fn modular_add_word(&mut self, root: usize, register: &[usize], value: &WordTape,
        modulus: &WordTape, controls: &[(usize,bool)]) -> usize {
        if !self.controls_possible(root,controls) { return root; }
        let value = value.divmod(modulus).expect("modular word addition requires a nonzero modulus").1;
        if value.is_zero() { return root; }
        VOX_MODULAR_COUNTERS[0].fetch_add(1,Ordering::Relaxed);
        VOX_MODULAR_COUNTERS[1].store(1,Ordering::Relaxed);
        VOX_MODULAR_COUNTERS[2].store(self.retained_nodes() as u64,Ordering::Relaxed);
        let zero = self.zero();
        let enabled = self.conditional_literals(zero,root,controls);
        let (valid,invalid) = self.partition_less_word(enabled,register,modulus);
        let threshold = modulus.sub(&value).unwrap();
        let (low,high) = self.partition_less_word(valid,register,&threshold);
        VOX_MODULAR_COUNTERS[1].store(2,Ordering::Relaxed);
        let low = self.add_constant_word(low,register,&value,&[]);
        VOX_MODULAR_COUNTERS[1].store(3,Ordering::Relaxed);
        let radix = WordTape::one_at(register.len());
        let high = self.add_constant_word(high,register,&radix.sub(&threshold).unwrap(),&[]);
        VOX_MODULAR_COUNTERS[1].store(4,Ordering::Relaxed);
        // The wrapped interval lands below value; the unwrapped interval
        // lands above it. Recovery therefore uses the transformed cut.
        let changed = self.fuse_interval_word(high, low, register, &value);
        VOX_MODULAR_COUNTERS[1].store(5,Ordering::Relaxed);
        let changed = self.fuse_interval_word(changed, invalid, register, modulus);
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
        let value = WordTape::from_biguint(value);
        fn visit(arena: &mut DecisionArena,id: usize,register: &[usize],value: &WordTape,
            at: usize,less: bool,flag: usize,memo: &mut OperationCache<(usize,usize,bool),usize>) -> usize {
            if at == register.len() { return if less { arena.flip(id,flag) } else { id }; }
            if matches!(arena.node(id),Node::Leaf(..)) { return id; }
            if let Some(&result) = memo.get(&(id,at,less)) { return result; }
            let wire = arena.top(id).unwrap().min(register[at]);
            let (low,high) = arena.split(id,wire);
            let (low,high) = if wire == register[at] {
                let constant = value.bit_at(at);
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
        let changed = visit(self,enabled,register,&value,0,false,flag,&mut OperationCache::default());
        self.conditional_literals(root,changed,controls)
    }
    fn mass_at(&self, root: usize, depth: usize,
        memo: &mut OperationCache<(usize, usize), WordTape>) -> WordTape {
        if let Some(mass) = memo.get(&(root, depth)) { return mass.clone(); }
        let mass = match self.node(root) {
            Node::Leaf(re, im) => re.magnitude().mul(re.magnitude())
                .add(&im.magnitude().mul(im.magnitude())).shl(self.cells - depth),
            &Node::Branch { wire, low, high } =>
                self.mass_at(low, wire + 1, memo).add(&self.mass_at(high, wire + 1, memo)).shl(wire - depth),
        };
        memo.insert((root, depth), mass.clone());
        mass
    }
    pub fn mass(&self, root: usize) -> BigUint {
        self.mass_word(root).to_biguint()
    }
    pub fn mass_word(&self, root: usize) -> WordTape {
        self.mass_at(root, 0, &mut OperationCache::default())
    }
    pub fn normalize(&mut self, root: usize, norm: &BigInt, scale: &BigInt) -> usize {
        fn visit(
            a: &mut DecisionArena,
            id: usize,
            norm: &SignedWordTape,
            scale: &SignedWordTape,
            memo: &mut OperationCache<usize, usize>,
        ) -> usize {
            if let Some(r) = memo.get(&id) {
                return *r;
            }
            let r = match a.node(id).clone() {
                Node::Leaf(re, im) => a.intern(Node::Leaf(re.mul(scale).div(norm), im.mul(scale).div(norm))),
                Node::Branch { wire, low, high } => {
                    let low = visit(a, low, norm, scale, memo);
                    let high = visit(a, high, norm, scale, memo);
                    a.branch(wire, low, high)
                }
            };
            memo.insert(id, r);
            r
        }
        visit(self, root, &SignedWordTape::from_bigint(norm), &SignedWordTape::from_bigint(scale),
            &mut OperationCache::default())
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

    #[cfg(feature = "hosted")]
    pub(crate) fn prepared_record(&self, root: usize) -> serde_json::Value {
        let mut order = Vec::new();
        let mut indices = OperationCache::default();
        let mut masks = Vec::new();
        let mut mask_indices = OperationCache::default();
        let mut fixed = Vec::new();
        let mut pending = alloc::vec![(root,false)];
        while let Some((id,returned)) = pending.pop() {
            if indices.contains_key(&id) { continue; }
            if !returned {
                pending.push((id,true));
                if let Node::Branch { low,high,.. } = self.node(id) {
                    pending.push((*high,false)); pending.push((*low,false));
                }
            } else {
                let node = match self.node(id) {
                    Node::Leaf(re,im) => serde_json::json!({"re":re.prepared_record(),"im":im.prepared_record()}),
                    Node::Branch { wire,low,high } => serde_json::json!({"wire":wire,"low":indices[low],"high":indices[high]}),
                };
                indices.insert(id,order.len()); order.push(node);
                let support = &self.fixed[id];
                let mut address = |mask: &SupportMask| {
                    let value = &**mask;
                    if let Some(&index) = mask_indices.get(value) { return index; }
                    let index = masks.len();
                    masks.push(value.to_u32_digits());
                    mask_indices.insert(value.clone(), index);
                    index
                };
                fixed.push((support.empty, address(&support.zero), address(&support.one)));
            }
        }
        serde_json::json!({"cells":self.cells,"root":indices[&root],"nodes":order,
            "support_masks":masks,"fixed":fixed})
    }

    #[cfg(feature = "hosted")]
    pub(crate) fn from_prepared_record(record: &serde_json::Value) -> Result<(Self,usize),String> {
        let index = |key: &str| record[key].as_u64().and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| alloc::format!("invalid prepared register {key}"));
        let mut arena = Self::new(index("cells")?);
        let mut nodes = Vec::new();
        for row in record["nodes"].as_array().ok_or("missing prepared register nodes")? {
            let node = if row.get("re").is_some() {
                arena.intern(Node::Leaf(SignedWordTape::from_prepared_record(&row["re"])?,
                    SignedWordTape::from_prepared_record(&row["im"])?))
            } else {
                let wire = row["wire"].as_u64().and_then(|n| usize::try_from(n).ok()).ok_or("invalid prepared cell address")?;
                let child = |key: &str| -> Result<usize,String> {
                    row[key].as_u64().and_then(|n| usize::try_from(n).ok()).and_then(|n| nodes.get(n)).copied()
                        .ok_or_else(|| alloc::format!("prepared {key} escaped its ancestral frame"))
                };
                let low = child("low")?; let high = child("high")?;
                if wire >= arena.cells || [low,high].into_iter().any(|id| arena.top(id).is_some_and(|top| top<=wire)) {
                    return Err("prepared register changed its cell ordering".into());
                }
                arena.branch(wire,low,high)
            };
            if let Some(fixed) = record.get("fixed") {
                let row = fixed.get(nodes.len()).ok_or("prepared support lost its node")?;
                let mask = |slot:usize| -> Result<BigUint,String> {
                    let id=row[slot].as_u64().and_then(|id|usize::try_from(id).ok())
                        .ok_or("invalid prepared support address")?;
                    let digits=record["support_masks"][id].as_array().ok_or("prepared support escaped its pool")?
                        .iter().map(|digit| digit.as_u64().and_then(|n|u32::try_from(n).ok())
                            .ok_or_else(||String::from("invalid prepared support limb")))
                        .collect::<Result<Vec<_>,String>>()?;
                    Ok(BigUint::new(digits))
                };
                let actual=&arena.fixed[node];
                if row[0].as_bool()!=Some(actual.empty) || mask(1)?!=*actual.zero || mask(2)?!=*actual.one {
                    return Err("prepared support differs from its amplitude diagram".into());
                }
            }
            nodes.push(node);
        }
        let root = *nodes.get(index("root")?).ok_or("prepared register lost its root")?;
        Ok((arena,root))
    }

    pub(crate) fn from_baked_nodes(cells:usize,root:usize,rows:&[crate::baked_resident::BakedNode],support:&crate::baked_resident::BakedSupport) -> Result<(Self,usize),String> {
        use crate::baked_resident::BakedNode;
        if support.fixed.len()!=rows.len() {return Err("compiled support lost its register binding".into());}
        let masks: Vec<_> = support.spans.iter().map(|&(start,end)| {
            let digits=support.limbs.get(start..end).ok_or("compiled mask escaped its limb block")?;
            let value=BigUint::new(digits.to_vec());
            Ok(if value.is_zero() {SupportMask(None)} else {SupportMask(Some(Arc::new(value)))})
        }).collect::<Result<_,String>>()?;
        let mut arena=Self::new(cells);
        arena.nodes.reserve(rows.len());
        arena.fixed.reserve(rows.len());
        arena.references.reserve(rows.len());
        arena.created.reserve(rows.len());
        let mut nodes=Vec::with_capacity(rows.len());
        for (row,&(empty,zero,one)) in rows.iter().zip(support.fixed) {
            let fixed=FixedBits {empty,
                zero:masks.get(zero).ok_or("compiled zero support escaped its pool")?.clone(),
                one:masks.get(one).ok_or("compiled one support escaped its pool")?.clone()};
            let node=match *row {
                BakedNode::Leaf(z)=>Node::Leaf(z.re.value()?,z.im.value()?),
                BakedNode::Branch {wire,low,high}=>{
                    let low=*nodes.get(low).ok_or("compiled low arm escaped its ancestry")?;
                    let high=*nodes.get(high).ok_or("compiled high arm escaped its ancestry")?;
                    if wire>=cells || [low,high].into_iter().any(|id|arena.top(id).is_some_and(|top|top<=wire)) {
                        return Err("compiled register changed its cell ordering".into());
                    }
                    if low==high {return Err("compiled register lost its reduced branch".into());}
                    Node::Branch {wire,low,high}
                }
            };
            if arena.unique.get(&node).is_some() {return Err("compiled register repeated a reduced node".into());}
            nodes.push(arena.intern_fixed(node,fixed));
        }
        let root=*nodes.get(root).ok_or("compiled register lost its root")?;
        Ok((arena,root))
    }
    #[cfg(test)]
    pub fn amplitude(&self, mut root: usize, value: &BigUint) -> (BigInt, BigInt) {
        loop {
            match self.node(root) {
                Node::Leaf(re, im) => return (re.to_bigint(), im.to_bigint()),
                &Node::Branch { wire, low, high } => {
                    root = if value.bit(wire as u64) { high } else { low }
                }
            }
        }
    }
}
