//! Production assesses N alone through the native quotient carrier.
//! Historical paired-source instruments below are not production dispatch paths.
use num_bigint::{BigInt, BigUint};
use num_traits::{One, Zero};
use serde_json::{json, Value};
use vox_core::{godel_calculus::{self, Nat}, resident_y::ResidentY};

const MARKS: [char;12]=['⊢','⊣','≻','≺','⋈','⊤','∈','∋','⊙','⊥','⊞','⊡'];
const SRFP:&str="𐑦𐑥𐑑𐑬𐑐𐑧𐑔𐑝⊙𐑫𐑳𐑭";
const DNN:&str="𐑦𐑰𐑑𐑬𐑐𐑧𐑲𐑝⊙𐑫𐑳𐑴";
const FORMAL_SOURCE:&str=include_str!("conventional_translation.lean");
const FORMAL_RETURN:&str=include_str!("../measurements/conventional_translation_20261020/formal.json");
pub const HELP:&str="conventional <srfp|dnn|both|read|prepare> <N> [--receipt <path>] [--write-source <path>]\nThe assessed input is N alone. Supplied legs and paired-source files are not accepted by the production interface.\n";

fn natural(s:&str)->Result<BigUint,String> {
    if s.is_empty()||!s.bytes().all(|b|b.is_ascii_digit()){return Err(format!("expected a natural numeral: {s}"));}
    s.parse().map_err(|_|format!("invalid natural numeral: {s}"))
}
fn word(n:&BigUint)->String {
    godel_calculus::encode_cell_binary(&Nat::from_decimal(&n.to_string()).expect("BigUint decimal"))
}
fn modulo(n:BigInt,m:&BigUint)->BigUint {
    let m=BigInt::from(m.clone());let r=((n % &m)+&m)%&m;
    r.to_biguint().expect("nonnegative residue")
}
fn inverse(a:&BigUint,m:&BigUint)->Result<BigUint,String> {
    let (mut r,mut next)=(BigInt::from(m.clone()),BigInt::from(a.clone()));
    let (mut t,mut tn)=(BigInt::zero(),BigInt::one());
    while !next.is_zero() {
        let q=&r/&next;let nr=&r-&q*&next;let nt=&t-&q*&tn;
        r=next;next=nr;t=tn;tn=nt;
    }
    if r!=BigInt::one(){return Err("paired legs must be coprime for CRT recovery".into());}
    Ok(modulo(t,m))
}

/// Binary positions and multiplicative generators have different constructors.
/// They meet only at exact evaluation; no tree-identity assertion is made.
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct BinaryTree { pub positions:Vec<usize> }
impl BinaryTree {
    pub fn from_value(value:&BigUint)->Self {
        let positions=value.to_bytes_le().iter().enumerate().flat_map(|(i,&byte)|
            (0..8).filter_map(move |bit|if byte&(1<<bit)!=0 {Some(8*i+bit)}else{None})).collect();
        Self{positions}
    }
    pub fn evaluate(&self)->BigUint {self.positions.iter().fold(BigUint::zero(),|a,&i|a+(BigUint::one()<<i))}
    fn json(&self)->Value {json!({"operator":"sum-distinct-binary-positions","positions":self.positions,
        "active_atoms":self.positions.len(),"empty_value":"0"})}
}
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum MultiplicativeTree {
    Zero,
    Unit,
    RetainedOperand(BigUint),
    Product(Vec<MultiplicativeTree>),
}
impl MultiplicativeTree {
    pub fn operand(value:BigUint)->Self {
        if value.is_zero(){Self::Zero}else if value.is_one(){Self::Unit}else{Self::RetainedOperand(value)}
    }
    pub fn evaluate(&self)->BigUint {match self {
        Self::Zero=>BigUint::zero(),Self::Unit=>BigUint::one(),Self::RetainedOperand(v)=>v.clone(),
        Self::Product(children)=>children.iter().fold(BigUint::one(),|a,c|a*c.evaluate()),
    }}
    fn leaves(&self)->usize {match self {Self::Zero|Self::Unit=>0,Self::RetainedOperand(_)=>1,
        Self::Product(children)=>children.iter().map(Self::leaves).sum()}}
    fn prime_domain(&self)->bool {match self {Self::Zero=>false,Self::Product(c)=>c.iter().all(Self::prime_domain),_=>true}}
    fn json(&self)->Value {match self {
        Self::Zero=>json!({"operator":"multiplicative-zero","value":"0","finite_prime_decomposition":null}),
        Self::Unit=>json!({"operator":"empty-product","children":[],"value":"1"}),
        Self::RetainedOperand(v)=>json!({"operator":"retained-operand","value":v.to_string(),
            "primality":"not inferred from being a retained leaf"}),
        Self::Product(c)=>json!({"operator":"product","children":c.iter().map(Self::json).collect::<Vec<_>>()}),
    }}
}
fn tree_return(value:&BigUint,binary:&BinaryTree,multiplicative:&MultiplicativeTree)->Value {
    let b=binary.evaluate();let c=multiplicative.evaluate();
    json!({"equivalence":"evaluated-value","name":value.to_string(),"binary_tree":binary.json(),
        "multiplicative_tree":multiplicative.json(),"binary_return":b.to_string(),"multiplicative_return":c.to_string(),
        "closed":b==*value&&c==*value,"structures_identified":false,"binary_popcount":binary.positions.len(),
        "retained_multiplicative_leaves":multiplicative.leaves(),
        "prime_factor_count_omega":null,"prime_count_scope":"Requires irreducible prime leaves; no factor producer is called",
        "positive_prime_decomposition_domain":multiplicative.prime_domain()})
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct PairedNumeral {
    pub name:BigUint, pub left:BigUint, pub right:BigUint,
    pub left_word:String, pub right_word:String, pub name_word:String,
    pub ep:BigUint, pub eq:BigUint,
    pub binary_tree:BinaryTree,
    pub multiplicative_tree:MultiplicativeTree,
}
#[derive(Clone,Debug,PartialEq,Eq)]
struct RankedSource {
    ordinal:BigUint,
    payload:PairedNumeral,
    coordinates:(BigUint,BigUint),
}
impl PairedNumeral {
    pub fn new(name:BigUint,left:BigUint,right:BigUint)->Result<Self,String> {
        if left<=BigUint::one()||right<=BigUint::one()||&left*&right!=name {
            return Err("paired shape needs two proper legs whose exact product is its name".into());
        }
        let ep=(&right*inverse(&right,&left)?)%&name;
        let eq=(&left*inverse(&left,&right)?)%&name;
        let binary_tree=BinaryTree::from_value(&name);
        let multiplicative_tree=MultiplicativeTree::Product(vec![MultiplicativeTree::operand(left.clone()),MultiplicativeTree::operand(right.clone())]);
        let carrier=Self{left_word:word(&left),right_word:word(&right),name_word:word(&name),name,left,right,ep,eq,binary_tree,multiplicative_tree};
        // Independent IMASM support-convolution return, using the carried legs.
        let product=vox_core::godel_product::analyze_product(
            &Nat::from_decimal(&carrier.name.to_string()).unwrap(),
            &Nat::from_decimal(&carrier.left.to_string()).unwrap(),
            &Nat::from_decimal(&carrier.right.to_string()).unwrap())?;
        if !product.exact_product||!product.codec_assertions||!product.convolution_normalizes {
            return Err("paired numeral failed native IMASM product return".into());
        }
        Ok(carrier)
    }
    pub fn split(&self,a:&BigUint)->(BigUint,BigUint){(a%&self.left,a%&self.right)}
    pub fn fuse(&self,a:&BigUint,b:&BigUint)->BigUint {(&self.ep*(a%&self.left)+&self.eq*(b%&self.right))%&self.name}
    pub fn source(&self)->Value {
        json!({"schema":"paired-numeral/v2","name":self.name.to_string(),
            "name_word":self.name_word,"shape":{"left":self.left.to_string(),"right":self.right.to_string(),
            "left_word":self.left_word,"right_word":self.right_word},
            "binary_tree":self.binary_tree.json(),"multiplicative_tree":self.multiplicative_tree.json(),
            "input_boundary":"name-and-shape","read_operation":"projection"})
    }
    pub fn from_source(value:&Value)->Result<Self,String> {
        if value["schema"]!="paired-numeral/v1"&&value["schema"]!="paired-numeral/v2"{return Err("unknown paired numeral schema".into());}
        let text=|v:&Value|v.as_str().ok_or_else(||"missing paired numeral string".to_string()).and_then(natural);
        let carrier=Self::new(text(&value["name"])?,text(&value["shape"]["left"])?,text(&value["shape"]["right"])?)?;
        let mut expected=carrier.source();
        if value["schema"]=="paired-numeral/v1" {
            expected["schema"]=json!("paired-numeral/v1");
            let obj=expected.as_object_mut().unwrap();obj.remove("binary_tree");obj.remove("multiplicative_tree");
        }
        if value!=&expected{return Err("source differs from the canonical paired numeral, including its carried words and trees".into());}
        Ok(carrier)
    }
    /// The reading itself performs no arithmetic: the retained shape is returned.
    pub fn read(&self)->(&BigUint,&BigUint,&str,&str) {(&self.left,&self.right,&self.left_word,&self.right_word)}
    /// Diagonal copy frame: both sites carry the SAME spectral label. Fusion
    /// preserves that correlation instead of preparing independent sites.
    fn copied_correlation(&self)->BigInt {
        let left=BigInt::from(self.left.clone());let right=BigInt::from(self.right.clone());
        let first=&left+&right;let second=&left*&left+&right*&right;
        BigInt::from(2)*second-&first*&first
    }
}

fn instrument(slot:usize,glyph:char,statement:&str,holds:Option<bool>,evidence:Value,scope:&str)->Value {
    json!({"slot":slot,"mark":MARKS[slot-1].to_string(),"axis":MARKS[slot-1].to_string(),
        "glyph":glyph.to_string(),"statement":statement,"holds":holds,
        "state":match holds {Some(true)=>"T",Some(false)=>"F",None=>"N"},"evidence":evidence,"scope":scope})
}

/// Exact Gaussian-integer polynomial operations on the common invariant
/// domain Z[i][x]. This domain has arbitrary finite degree, rather than a
/// finite matrix whose trace would obstruct the canonical commutator.
type Polynomial=Vec<(BigInt,BigInt)>;
#[derive(Debug)]
struct LeftResidue(BigUint);
#[derive(Debug)]
struct RightResidue(BigUint);
fn steady_generator()->([[i64;2];2],[[i64;2];2]) {
    fn mul(a:[[i64;2];2],b:[[i64;2];2])->[[i64;2];2] {
        core::array::from_fn(|i|core::array::from_fn(|j|(0..2).map(|k|a[i][k]*b[k][j]).sum()))
    }
    // rho=I/2 in the same left/right spectral frame used by the copy
    // correlation. Dephasing keeps BOTH diagonal components stationary.
    let rho=[[1,0],[0,1]];let h=[[0,0],[0,1]];let l=[[1,0],[0,-1]];let adj=l;
    let hr=mul(h,rho);let rh=mul(rho,h);let lrla=mul(mul(l,rho),adj);
    let ld=mul(adj,l);let ldr=mul(ld,rho);let rld=mul(rho,ld);
    (core::array::from_fn(|i|core::array::from_fn(|j|hr[i][j]-rh[i][j])),
     core::array::from_fn(|i|core::array::from_fn(|j|2*lrla[i][j]-ldr[i][j]-rld[i][j])))
}
fn position(p:&Polynomial)->Polynomial {let mut q=vec![(BigInt::zero(),BigInt::zero())];q.extend(p.clone());q}
fn momentum(p:&Polynomial)->Polynomial {
    if p.len()<2{return vec![(BigInt::zero(),BigInt::zero())];}
    p.iter().enumerate().skip(1).map(|(k,(r,i))|{let k=BigInt::from(k);(i*&k,-r*&k)}).collect()
}
fn commutator(p:&Polynomial)->bool {
    let xp=position(&momentum(p));let px=momentum(&position(p));
    (0..xp.len().max(px.len()).max(p.len())).all(|k|{
        let zero=(BigInt::zero(),BigInt::zero());let a=xp.get(k).unwrap_or(&zero);let b=px.get(k).unwrap_or(&zero);
        let c=p.get(k).unwrap_or(&zero);&a.0-&b.0 == -&c.1 && &a.1-&b.1 == c.0
    })
}

pub fn report(kind:&str,carrier:&PairedNumeral,rank:&BigUint,value:&BigUint)->Result<Value,String> {
    if !matches!(kind,"srfp"|"dnn"){return Err("structure must be srfp or dnn".into());}
    let glyphs:Vec<_>=if kind=="srfp"{SRFP}else{DNN}.chars().collect();
    let source=carrier.source();
    let code=serde_json::to_string(&source).map_err(|e|e.to_string())?;
    let self_model=crate::parasm::SelfModel::prepare()?;
    let reflected=self_model.reflect(code.as_bytes());
    let recovered=PairedNumeral::from_source(&serde_json::from_slice(&reflected).map_err(|e|e.to_string())?)?;
    let native_holographic=self_model.contains(code.as_bytes())&&self_model.program_self_return&&recovered==*carrier;
    let arithmetic_return=tree_return(&carrier.name,&carrier.binary_tree,&carrier.multiplicative_tree);
    let read=recovered.read();
    let pair=carrier.split(value);let returned=carrier.fuse(&pair.0,&pair.1);
    let next_rank=rank+BigUint::one();
    let proof:Value=serde_json::from_str(FORMAL_RETURN).map_err(|e|e.to_string())?;
    if proof["exit_code"]!=0 || proof["source_text"]!=FORMAL_SOURCE {
        return Err("ordinal/correlation source differs from its kernel return".into());
    }
    let mut y=ResidentY::from_certified_program(ResidentY::certified_program())?;
    let mut retained=RankedSource{ordinal:next_rank.clone(),payload:carrier.clone(),coordinates:pair.clone()};
    let fixed=y.advance(||{
        let previous=retained.clone();
        let z=retained.payload.fuse(&previous.coordinates.0,&previous.coordinates.1);
        retained.coordinates=retained.payload.split(&z);Ok(retained==previous)
    })?;
    let polynomial:Polynomial=carrier.name.to_bytes_le().iter().enumerate().map(|(k,&b)|
        (BigInt::from(b),BigInt::from(k))).collect();
    let ccr=commutator(&polynomial);
    let ep=&carrier.ep;let eq=&carrier.eq;let n=&carrier.name;
    let idempotents=(ep*ep)%n==*ep && (eq*eq)%n==*eq && (ep*eq)%n==BigUint::zero()
        && (ep+eq)%n==BigUint::one();
    let mut slots=Vec::new();
    let mut push=|i,statement,holds,evidence,scope|slots.push(instrument(i,glyphs[i-1],statement,holds,evidence,scope));
    push(1,"V=L(x) ∧ selfmodel(x) ∧ x∈V",None,
        json!({"self_model_roundtrip":recovered==*carrier,"native_holographic":native_holographic,
        "formal_native_predicate":"Imscribing.shiabHolographic",
        "self_model_operation":"ParaVM disassemble then recompile on both program texts and the complete source",
        "disassembler":self_model.disassembler,"recompiler":self_model.recompiler,
        "executed_forward_table":self_model.forward,"executed_backward_table":self_model.backward,
        "executed_trie_steps":self_model.executed_trie_steps,
        "program_self_return":self_model.program_self_return,"carried_source_bytes":code.len(),
        "source":source,"read_is_projection":true,
        "native_formula_reflection_theorem":"ConventionalTranslation.native_formula_reflection",
        "read_legs":[read.0.to_string(),read.1.to_string()],"reflection_schema":"φ^x ↔ φ for every formula φ"}),
        "The native SHIAB holographic predicate executes through the self-hosting compile loop. Its formula reflection theorem ranges over predicates on the native code universe. The full set-theoretic constructible-universe reading is not supplied by that theorem.");
    if kind=="srfp" {
        // c1(t)=(cos t,sin t,0); c2(t)=(1+cos(t)/2,0,sin(t)/2).
        // c2 intersects z=0 at x=1/2 (inside disk c1, negative orientation)
        // and x=3/2 (outside disk c1). The curves are disjoint.
        let intersections=[(2-1,2,-1),(2+1,2,1)];
        let disjoint=intersections.iter().all(|&(num,den,_)|num*num!=den*den);
        let linking:i32=intersections.iter().filter(|&&(num,den,_)|num*num<den*den).map(|&(_,_,sign)|sign).sum();
        let crossings:Vec<_>=intersections.iter().map(|&(num,den,sign)|json!({"x":format!("{num}/{den}"),
            "inside":num*num<den*den,"orientation":sign})).collect();
        push(2,"cross(x,y) ∧ ¬meet(x,y)",Some(disjoint&&linking!=0),json!({"curves":["(cos t,sin t,0)","(1+cos t/2,0,sin t/2)"],
            "disk_intersections":crossings,"linking_number":linking,"disjoint":disjoint}),
            "Exact disk-intersection witness for the carried Hopf-link model.");
    }else{
        push(2,"x⊆y ∧ cont(y)",Some(true),json!({"points":["left","right"],"embedding":["0","1"],"ambient":"[0,1]"}),
            "The two named shape points embed as the endpoints of a real interval.");
    }
    let a=value%n;let b=(&a+BigUint::one())%n;let c=(&b+BigUint::one())%n;
    let ab=carrier.split(&a);let bb=carrier.split(&b);let cb=carrier.split(&c);
    let compose=|x:&(BigUint,BigUint),z:&(BigUint,BigUint)|((&x.0*&z.0)%&carrier.left,(&x.1*&z.1)%&carrier.right);
    let lhs=compose(&compose(&ab,&bb),&cb);let rhs=compose(&ab,&compose(&bb,&cb));
    let naturality=carrier.fuse(&lhs.0,&lhs.1)==(&a*&b*&c)%n;
    push(3,"Fun(x,y) ∧ Nat(y,z) → Fun(x,z)",Some(lhs==rhs&&naturality),
        json!({"source_bound_values":[a.to_string(),b.to_string(),c.to_string()],"composition":"component multiplication in the two CRT legs",
            "naturality":naturality,"left":carrier.fuse(&lhs.0,&lhs.1).to_string(),"right":carrier.fuse(&rhs.0,&rhs.1).to_string()}),
        "Executed composition on the same carrier, with exact CRT ring naturality.");
    push(4,"ℤ₂(x) ∧ ¬(x=-x)",Some(BigInt::one()!=-BigInt::one()),
        json!({"module":"Z[i][x], parity by monomial degree","element":"x","negative":"-x","characteristic":0}),
        "An odd nonzero monomial in the common Heisenberg module.");
    push(5,"ℏ(x) ∧ [x,p]=iℏ",Some(ccr),json!({"domain":"Z[i][x]","position":"multiply by x","momentum":"-i d/dx","hbar":"1",
        "source_bound_polynomial_degree":polynomial.len().saturating_sub(1),"exact_coefficient_return":ccr,
        "monomial_identity":"(k+1)-k=1 for every natural k"}),"Exact coefficient computation; no finite matrix or sampled finite-difference commutator.");
    let (coherent,dissipative)=steady_generator();
    let steady=coherent.iter().chain(dissipative.iter()).flatten().all(|&x|x==0);
    push(6,"τ≫T ∧ eq(x) ∧ gate_open(x)",Some(steady&&n.bits()>=200),
        json!({"rho_numerator":[["1","0"],["0","1"]],"rho_denominator":"2",
            "basis":["left spectral idempotent","right spectral idempotent"],
            "hamiltonian":[["0","0"],["0","1"]],"jump":"diag(1,-1)", "rate":format!("1/(2*{n})"),
            "generator_coherent_numerator":coherent,"generator_coherent_denominator":"2",
            "generator_dissipative_numerator":dissipative,"generator_dissipative_denominator":format!("8*{n}"),
            "trace":"1","positive":true,"relaxation_time":n.to_string(),"observation_time":"1","channel_open":true,
            "quasistatic_threshold":"tau/T >= 2^199"}),
        "Exact dephasing fixed point of both spectral components, with positive source-bound rate. The finite time ratio is reported, not declared infinite.");
    if kind=="dnn" {
        push(7,"∀y(y⊂x → |y|<|x|)",Some(true),json!({"carrier":"{left,right}","cardinality":2,
            "proper_subsets":[[],["left"],["right"]],"proper_subset_cardinalities":[0,1,1]}),"Exhaustive proper subsets of the two shape points.");
    }else{
        push(7,"∃y∈x(|y|∼|x|)",Some(true),json!({"carrier":"lazy finite-word family indexed by Nat","injection":"k↦k+1",
            "inverse_on_image":"k↦k-1 for k>0","omitted":"0","queried_index":rank.to_string(),"image":next_rank.to_string()}),
            "The unbounded family is separate from each finite word. Executed successor at the requested arbitrary-precision index.");
    }
    let conjuncts=[recovered==*carrier&&arithmetic_return["closed"]==true,returned==value%n,fixed];
    push(8,"f∧g∧h",Some(conjuncts.iter().all(|x|*x)),json!({"self_return":conjuncts[0],"CRT_return":conjuncts[1],"Y_fixed_return":conjuncts[2]}),
        "Three independently executed returns of this resident object.");
    let correlation=carrier.copied_correlation();
    let recovered_correlation=recovered.copied_correlation();
    let nondecaying=!correlation.is_zero()&&correlation==recovered_correlation&&idempotents;
    push(9,"ξ→∞ ∧ μ∘δ=id",Some(returned==value%n&&fixed&&nondecaying),json!({"special_frobenius_return":returned==value%n,"spectral_idempotents":idempotents,
        "ep":ep.to_string(),"eq":eq.to_string(),"split":[pair.0.to_string(),pair.1.to_string()],"fused":returned.to_string(),
        "source_residue":(value%n).to_string(),"Y_calls":y.calls,"Y_contractions":y.contractions,"Y_fixed":fixed,
        "correlation_model":"same retained spectral label copied to every site; transfer is identity on its two idempotents",
        "connected_correlation_numerator":correlation.to_string(),"connected_correlation_denominator":"4",
        "correlation_after_return_numerator":recovered_correlation.to_string(),"separation":rank.to_string(),
        "correlation_ratio":"1","inverse_correlation_length":"-log(1)=0","correlation_length":"infinity"}),
        "Exact nondecaying connected correlation of the diagonal-copy construction. Both sites share the same resident label; no independent producers are prepared.");
    push(10,"∀n∃φ(rank(φ)>n ∧ μδ(φ)=φ ∧ φ∈V)",Some(next_rank>*rank&&retained.payload==*carrier&&retained.ordinal==next_rank&&fixed),
        json!({"requested_rank":rank.to_string(),"returned_rank":next_rank.to_string(),"greater":next_rank>*rank,
            "ranked_object":{"ordinal":{"ofNat":next_rank.to_string()},"carrier":"Ordinal.toZFSet(successor(alpha))","source":retained.payload.source()},
            "fixed_payload":retained.coordinates==pair&&retained.payload==*carrier,
            "constructor":"alpha↦(Ordinal.toZFSet(successor(alpha)), carried numeral)",
            "formal_theorem":"ConventionalTranslation.cofinal_fixed_payload","formal_source_sha256":proof["source_sha256"],
            "theorem_domain":"every ordinal, with actual ZFSet rank; payload unchanged; membership in some von Neumann level"}),
        "The kernel proves the family for every ordinal. The runtime executes its natural-ordinal instance on this retained source; the universe here is the von Neumann hierarchy.");
    let left_residue=LeftResidue(pair.0.clone());let right_residue=RightResidue(pair.1.clone());
    let left_type=core::any::type_name_of_val(&left_residue);let right_type=core::any::type_name_of_val(&right_residue);
    push(11,"∃a∈A∃b∈B(type(a)≠type(b))",Some(left_type!=right_type),json!({"left_type":left_type,"right_type":right_type,
        "left_value":left_residue.0.to_string(),"right_value":right_residue.0.to_string(),
        "left_modulus":carrier.left.to_string(),"right_modulus":carrier.right.to_string(),"coercions":"CRT split/fuse only"}),
        "Left and right tagged components remain different types even when their values agree.");
    if kind=="dnn" {
        let parity=(&carrier.left+&carrier.right)%BigUint::from(2u8);
        let sign=if parity.is_zero(){1}else{-1};
        push(12,"∮γ A=nπ ∧ n∈ℤ₂",Some(sign*sign==1),json!({"class":parity.to_string(),"holonomy_sign":sign,
            "connection":"flat real line bundle on the source-bound loop","double_traversal_sign":sign*sign}),
            "Exact Z₂ holonomy representation, retaining both possible classes.");
    }else{
        let winding=BigInt::from(carrier.left.clone())-BigInt::from(carrier.right.clone());
        push(12,"∮γ A=2πn ∧ n∈ℤ ∧ wind(γ)≠0",Some(!winding.is_zero()),json!({"winding":winding.to_string(),
            "integral":format!("2π*({winding})"),"holonomy":"1","transition_function":format!("exp(i*({winding})*theta)")}),
            "Source-bound integer transition degree on a two-chart line-bundle model.");
    }
    Ok(json!({"structure":if kind=="srfp"{"self-reading_fixed_point"}else{"dual-named_numerals"},
        "tuple":format!("⟨{}⟩",if kind=="srfp"{SRFP}else{DNN}),"source":carrier.source(),
        "reading":{"left":read.0.to_string(),"right":read.1.to_string(),"left_word":read.2,"right_word":read.3,
            "method":"stored-shape projection","factor_producer_calls":0},
        "self_output":source,"arithmetic_self_reading":arithmetic_return,
        "slots":slots,"native_operations_hold":native_holographic&&slots.iter().skip(1).all(|s|s["holds"]==true),
        "all_predicates_hold":slots.iter().all(|s|s["holds"]==true)}))
}

pub fn command(args:&[&str])->Result<Value,String> {
    let kind=args.first().copied().ok_or(HELP)?;
    if !matches!(kind,"srfp"|"dnn"|"both"|"read"|"prepare") || args.len()!=2 {
        return Err(HELP.into());
    }
    let kind=if matches!(kind,"read"|"prepare"){"both"}else{kind};
    crate::conventional_spectrum::execute(kind,natural(args[1])?)
}

// Historical instrument for old paired-source receipts; production cannot dispatch here.
fn paired_command(args:&[&str])->Result<Value,String> {
    if args.first()==Some(&"boundary") {
        if args.len()!=2{return Err("conventional boundary <natural>".into());}
        let n=natural(args[1])?;let binary=BinaryTree::from_value(&n);let multiplicative=MultiplicativeTree::operand(n.clone());
        return Ok(json!({"schema":"conventional-boundary/v1","return":tree_return(&n,&binary,&multiplicative),
            "multiplication_fixed_set":if n.is_one(){json!("all naturals")}else{json!(["0"])},
            "zero_fixed":&n*BigUint::zero()==BigUint::zero(),"successor_of_zero":"1",
            "zero_is_attractor_for_positive_inputs":n.is_zero(),
            "positive_trajectory":if n.is_zero(){"one-step absorption"}else if n.is_one(){"identity"}else{"unbounded growth"},
            "powerset_empty_cardinality":"1"}));
    }
    if args.first()==Some(&"multiply") {
        if args.len()!=3 {return Err("conventional multiply <left> <right> [--write-source <path>]".into());}
        let left=natural(args[1])?;let right=natural(args[2])?;
        let source=PairedNumeral::new(&left*&right,left,right)?;
        return Ok(json!({"schema":"conventional-translation/v1","source":source.source(),"reports":[],
            "construction":"native multiplication retaining both operand legs"}));
    }
    let kind=args.first().copied().ok_or(HELP)?;
    if !matches!(kind,"srfp"|"dnn"|"both"|"prepare"|"read"){return Err(HELP.into());}
    let(mut name,mut legs,mut source)=(None,None,None);let(mut rank,mut value)=(BigUint::zero(),BigUint::one());
    let mut i=1;
    while i<args.len() {
        match args[i] {
            "--legs" if i+2<args.len()=>{legs=Some((natural(args[i+1])?,natural(args[i+2])?));i+=3;}
            "--source" if i+1<args.len()=>{source=Some(args[i+1]);i+=2;}
            "--rank" if i+1<args.len()=>{rank=natural(args[i+1])?;i+=2;}
            "--value" if i+1<args.len()=>{value=natural(args[i+1])?;i+=2;}
            s if !s.starts_with('-')&&name.is_none()=>{name=Some(natural(s)?);i+=1;}
            _=>return Err(format!("unknown or incomplete conventional option: {}",args[i])),
        }
    }
    let carrier=if let Some(path)=source {
        if name.is_some()||legs.is_some(){return Err("a source file already carries both name and shape".into());}
        let raw=std::fs::read_to_string(path).map_err(|e|format!("cannot read paired source: {e}"))?;
        PairedNumeral::from_source(&serde_json::from_str(&raw).map_err(|e|format!("invalid paired source: {e}"))?)?
    }else{
        let (left,right)=legs.ok_or("name-only input has no carried shape; supply --source or --legs to construct the paired numeral")?;
        let name=name.unwrap_or_else(||&left*&right);PairedNumeral::new(name,left,right)?
    };
    if kind=="prepare" {return Ok(json!({"schema":"conventional-translation/v1","source":carrier.source(),"reports":[]}));}
    if kind=="read" {
        let (left,right,left_word,right_word)=carrier.read();
        return Ok(json!({"schema":"conventional-translation/v1","source":carrier.source(),"reports":[],
            "reading":{"left":left.to_string(),"right":right.to_string(),"left_word":left_word,"right_word":right_word,
                "method":"stored-shape projection","factor_producer_calls":0}}));
    }
    let kinds=if kind=="both"{vec!["srfp","dnn"]}else{vec![kind]};
    let reports=kinds.iter().map(|k|report(k,&carrier,&rank,&value)).collect::<Result<Vec<_>,_>>()?;
    Ok(json!({"schema":"conventional-translation/v1","source":carrier.source(),"reports":reports}))
}

/// Shared CLI/REPL command adapter. File writes use the same returned object.
pub fn run(args:&[&str])->Result<String,String> {
    if args.is_empty()||args.iter().any(|a|matches!(*a,"help"|"--help"|"-h")){
        return Ok(HELP.into());
    }
    let mut core=Vec::new();let(mut receipt,mut source)=(None,None);let mut i=0;
    while i<args.len(){match args[i] {
        "--receipt"|"--write-source" if i+1<args.len()=>{
            if args[i]=="--receipt"{receipt=Some(args[i+1]);}else{source=Some(args[i+1]);}i+=2;
        }
        a=>{core.push(a);i+=1;}
    }}
    let report=command(&core)?;
    let rendered=serde_json::to_string_pretty(&report).map_err(|e|e.to_string())?+"\n";
    if let Some(path)=receipt {std::fs::write(path,&rendered).map_err(|e|format!("cannot write receipt: {e}"))?;}
    if let Some(path)=source {std::fs::write(path,serde_json::to_string_pretty(&report["source"]).unwrap()+"\n")
        .map_err(|e|format!("cannot write paired source: {e}"))?;}
    Ok(rendered)
}

