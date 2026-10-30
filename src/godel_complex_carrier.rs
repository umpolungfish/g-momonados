//! Exact complex lift of the native prepared Gödel graph. The held object is
//! a factored amplitude function, not independent candidate producers.
use crate::{idempotent_projector::Projector,ququart_decision::DecisionArena,
    word_tape::WordTape,anyon_ququart::FixedQuquartSic,phase_unbraid::FixedPointFormat};
use num_bigint::{BigInt,BigUint};
use num_traits::{One,Zero};
use serde_json::{Value,json};
use std::collections::{BTreeMap,BTreeSet};

pub struct GodelComplexCarrier {
    source:WordTape,
    arena:DecisionArena,
    projector:Option<Projector>,
    source_bank:usize,
    initial_bank:usize,
    frame_banks:Vec<usize>,
    rotation_base:Vec<char>,rotation_modulus:Vec<char>,rotation_residue:Vec<char>,rotation_order:Vec<char>,rotation_closed:bool,
    p:Vec<usize>,q:Vec<usize>,
    boundary:usize,
    graph:Value,
    catalyst:Value,
    frame:FixedQuquartSic,
    steps:usize,
    exchanges:Vec<(usize,Vec<usize>,Vec<usize>)>,
    certificate_pool:BTreeMap<String,Value>,
    chart_cache:BTreeMap<String,String>,
    closed:Option<usize>,
    resolved:Option<(WordTape,WordTape)>,
}

fn indices(value:&Value,key:&str)->Result<Vec<usize>,String>{
    serde_json::from_value(value[key].clone()).map_err(|e|format!("invalid carrier {key}: {e}"))
}
fn integer(value:&Value,key:&str)->Result<usize,String>{
    value[key].as_u64().and_then(|x|usize::try_from(x).ok()).ok_or_else(||format!("invalid carrier {key}"))
}
fn validate_cycle(c:&Value)->Result<i32,String>{
    if c["format"]!="modot-executed-cycle-v1"||c["regenerated"]!=true||c["resting"]!=c["returned"]{
        return Err("carrier requires an executed, regenerated MoDoT cycle".into());
    }
    let read=|key:&str,at:usize|c[key][at].as_i64().ok_or_else(||format!("missing cycle {key}[{at}]"));
    let delta=read("product",11)?-read("input",11)?;
    let spent=read("spent",11)?-read("resting",11)?;
    if delta.abs()!=1||spent!=-delta||c["terminal_exchange"].as_i64()!=Some(delta){
        return Err("cycle lost its winding transfer or terminal return".into());
    }
    if c["imprint"]!="R↔S"{return Err("numerical lift currently requires the executed R/S imprint".into());}
    let mut orientation=0;
    for (a,b) in [(2,10)]{
        let ma=c["ordinal_maxima"][a].as_i64().ok_or("missing cycle scale")?+1;
        let mb=c["ordinal_maxima"][b].as_i64().ok_or("missing cycle scale")?+1;
        let da=(read("product",a)?-read("input",a)?).rem_euclid(ma);
        let db=(read("product",b)?-read("input",b)?).rem_euclid(mb);
        if !((da==1&&db==mb-1)||(da==ma-1&&db==1)){
            return Err("cycle lost its conjugate structural imprint".into());
        }
        orientation=if da==1{1}else{-1};
    }
    Ok(delta as i32*orientation)
}

fn advance_relative_rotation(base:&[char],modulus:&[char],residue:&mut Vec<char>,order:&mut Vec<char>)->bool{
    *residue=vox_core::morphism_factor::modulo(&vox_core::morphism_factor::mul(residue,base),modulus);
    *order=vox_core::morphism_factor::add(order,&vox_core::morphism_factor::one());
    vox_core::morphism_factor::cmp(residue,&vox_core::morphism_factor::one())==core::cmp::Ordering::Equal
}
fn assign_factor_lanes<'a>(p:&'a[usize],q:&'a[usize],a:&'a BigUint,b:&'a BigUint)->Result<[(&'a[usize],&'a BigUint);2],String>{
    if a.bits()<=p.len() as u64&&b.bits()<=q.len() as u64{return Ok([(p,a),(q,b)]);}
    if b.bits()<=p.len() as u64&&a.bits()<=q.len() as u64{return Ok([(p,b),(q,a)]);}
    Err("exact factor pair exceeds the retained lane widths".into())
}
fn decode_tape(tape:&[char])->BigUint{let mut bytes=vec![0u8;(tape.len()+7)/8];for(i,mark)in tape.iter().enumerate(){if *mark=='⊥'{bytes[i/8]|=1<<(i%8);}}BigUint::from_bytes_le(&bytes)}

fn append_table(tables:&mut Vec<(Vec<usize>,Vec<usize>)>,wires:&[usize],assignments:Vec<Vec<bool>>){
    let mut order:Vec<_>=(0..wires.len()).collect();order.sort_by_key(|&i|wires[i]);
    let scope:Vec<_>=order.iter().map(|&i|wires[i]).collect();
    let rows=assignments.into_iter().map(|values|order.iter().enumerate().fold(0usize,|row,(at,&old)|
        row|(usize::from(values[old])<<at))).collect();
    tables.push((scope,rows));
}

fn append_mod12_residue(tables:&mut Vec<(Vec<usize>,Vec<usize>)>,next:&mut usize,bits:&[usize])->[usize;4]{
    let mut state=[0usize;4];for cell in &mut state{*cell=*next;*next+=1;}
    append_table(tables,&state,vec![vec![false;4]]);
    let mut weight=1usize;
    for &input in bits{
        let mut output=[0usize;4];for cell in &mut output{*cell=*next;*next+=1;}
        let mut wires=state.to_vec();wires.push(input);wires.extend(output);
        let mut rows=Vec::new();
        for residue in 0..12{for bit in 0..=1{
            let result=(residue+bit*weight)%12;
            let mut row=Vec::with_capacity(9);
            row.extend((0..4).map(|i|residue&(1<<i)!=0));row.push(bit!=0);
            row.extend((0..4).map(|i|result&(1<<i)!=0));rows.push(row);
        }}
        append_table(tables,&wires,rows);state=output;weight=(weight*2)%12;
    }
    state
}

/// Lift the factor pair's modulo-12 address into the exact 144-outcome SIC
/// dual relation. The small automata read the existing factor bits; no factor
/// value or width enters from outside the N-bound graph.
fn append_d12_sic_relation(
    tables:&mut Vec<(Vec<usize>,Vec<usize>)>,next:&mut usize,p:&[usize],q:&[usize],source_residue:usize,
)->Result<(usize,usize),String>{
    let constraints_before=tables.len();let first_aux=*next;
    let p_residue=append_mod12_residue(tables,next,p);
    let q_residue=append_mod12_residue(tables,next,q);
    let mut product_wires=p_residue.to_vec();product_wires.extend(q_residue);
    let mut product_rows=Vec::new();
    for a in 0..12{for b in 0..12{if (a*b)%12==source_residue{
        let mut row=Vec::with_capacity(8);row.extend((0..4).map(|i|a&(1<<i)!=0));
        row.extend((0..4).map(|i|b&(1<<i)!=0));product_rows.push(row);
    }}}
    append_table(tables,&product_wires,product_rows);
    let mut p_equal=[0usize;12];let mut q_equal=[0usize;12];
    for cell in p_equal.iter_mut().chain(q_equal.iter_mut()){*cell=*next;*next+=1;}
    for value in 0..12{
        for (residue,flag) in [(&p_residue,&p_equal[value]),(&q_residue,&q_equal[value])]{
            let mut wires=residue.to_vec();wires.push(*flag);let mut rows=Vec::new();
            for r in 0..16{for out in [false,true]{if out==(r==value){
                let mut row=(0..4).map(|i|r&(1<<i)!=0).collect::<Vec<_>>();row.push(out);rows.push(row);
            }}}
            append_table(tables,&wires,rows);
        }
    }
    for a in 0..12{for b in 0..12{
        let selector=*next;*next+=1;let wires=[p_equal[a],q_equal[b],selector];
        append_table(tables,&wires,vec![vec![false,false,false],vec![false,true,false],vec![true,false,false],vec![true,true,true]]);
        let mut mass=[0usize;4];for cell in &mut mass{*cell=*next;*next+=1;}
        let mut coordinates=[0usize;8];for cell in &mut coordinates{*cell=*next;*next+=1;}
        let mut dual_wires=vec![selector];dual_wires.extend(mass);dual_wires.extend(coordinates);
        let mut rows=Vec::with_capacity(2);for (selected,mass_value,coordinate) in [(false,1usize,0usize),(true,13usize,156usize)]{
            let mut row=vec![selected];row.extend((0..4).map(|i|mass_value&(1<<i)!=0));
            row.extend((0..8).map(|i|coordinate&(1<<i)!=0));rows.push(row);
        }
        append_table(tables,&dual_wires,rows);
    }}
    let cell_count=*next-first_aux;let constraint_count=tables.len()-constraints_before;
    if cell_count==0||constraint_count==0{return Err("d=12 SIC relation did not enter the shared arena".into());}
    Ok((cell_count,constraint_count))
}

fn live_clink_l9_word()->Result<String,String>{
    let home=std::env::var_os("HOME").ok_or("HOME is unset; cannot read the referenced imscription catalog")?;
    let path=std::path::PathBuf::from(home).join(".imscrbgrmr/catalog.json");
    let catalog:Value=serde_json::from_slice(&std::fs::read(path).map_err(|e|format!("read referenced catalog: {e}"))?)
        .map_err(|e|format!("parse referenced catalog: {e}"))?;
    let entries=catalog["imscriptions"].as_array().ok_or("referenced catalog omitted imscriptions")?;
    let entry=entries.iter().find(|entry|entry["name"]=="clink_l9").ok_or("referenced catalog omitted clink_l9")?;
    let order=["⊢","⊣","≻","≺","⋈","⊤","∈","∋","⊙","⊥","⊞","⊡"];
    let mut word=String::new();
    for key in order{word.push_str(entry[key].as_str().ok_or("clink_l9 tuple omitted a primitive")?);}
    if word.chars().count()!=12||word.chars().nth(8)!=Some('⊙'){
        return Err("referenced clink_l9 does not carry its ⊙ criticality slot".into());
    }
    Ok(word)
}

/// Install the live CLINK L9 Frobenius bottleneck on every shared factor bit.
/// Each gate retains the split arms, the ⊙ criticality cell, an EVALT work
/// cell, and the fused return on the same boundary factor coordinate.
fn append_clink_l9_relation(
    tables:&mut Vec<(Vec<usize>,Vec<usize>)>,next:&mut usize,factor_cells:&[usize],
)->Result<(usize,usize),String>{
    if factor_cells.is_empty(){return Err("CLINK L9 has no shared factor coordinates".into());}
    let before=tables.len();let first=*next;
    for &source in factor_cells{
        let mut cells=[0usize;5];for cell in &mut cells{*cell=*next;*next+=1;}
        let [left,right,critical,work,fused]=cells;
        let wires=[source,left,right,critical,work,fused];
        let rows=[false,true].into_iter().map(|v|vec![v,v,v,true,true,v]).collect();
        append_table(tables,&wires,rows);
        append_table(tables,&[critical],vec![vec![true]]);
        append_table(tables,&[work],vec![vec![true]]);
    }
    Ok((*next-first,tables.len()-before))
}

impl GodelComplexCarrier {
    pub fn prepare(source:&str,catalyst:Value)->Result<Self,String>{
        let preparation_clock=std::time::Instant::now();
        validate_cycle(&catalyst)?;
        let n=BigUint::parse_bytes(source.as_bytes(),10).ok_or("invalid N-only decimal")?;
        if n.bits()<100||!n.bit(0){return Err("complex Gödel production requires an odd source of at least 100 bits".into());}
        let source=WordTape::from_biguint(&n);let tape=source.parity_cells();
        let (relation,type_record)=
            vox_core::glut_system::prepare_full_device_correlation_with_dialectic(&tape)?;
        eprintln!("godel-complex preparation native-graph seconds={:.6} cells={} gates={} type-frontier={} y-contractions={} y-descents={}",
            preparation_clock.elapsed().as_secs_f64(),relation.masks.len(),relation.gates.len(),
            type_record.frontier.as_ref().map_or(0,Vec::len),type_record.operations.contractions,type_record.operations.descents);
        if relation.empty{return Err("prepared Gödel relation is empty".into());}
        let fixed:Vec<_>=relation.belnap_domains.iter().map(|domain|
            domain.only().and_then(|value|value.definite())).collect();
        let factor_cells:BTreeSet<_>=relation.p.iter().chain(&relation.q).copied().collect();
        let mut order:Vec<_>=factor_cells.iter().copied().collect();
        let count=order.len();order.extend((0..relation.masks.len()).filter(|w|!factor_cells.contains(w)));
        let mut map=vec![0;order.len()];for (new,&old) in order.iter().enumerate(){map[old]=tape.len()+new;}
        let boundary=tape.len()+count;
        let p:Vec<usize>=relation.p.iter().map(|&w|map[w]).collect();
        let q:Vec<usize>=relation.q.iter().map(|&w|map[w]).collect();
        let mut tables=Vec::new();
        let mut resident_cells=tape.len()+map.len();
        let source_residue=(&n%BigUint::from(12u8)).to_u32_digits().first().copied().unwrap_or(0) as usize;
        let (sic_cells,sic_constraints)=append_d12_sic_relation(&mut tables,&mut resident_cells,&p,&q,source_residue)?;
        let clink_word=live_clink_l9_word()?;
        let clink_criticality=clink_word.chars().nth(8).ok_or("clink_l9 lost its criticality slot")?;
        let shared_factor_cells:BTreeSet<_>=p.iter().chain(&q).copied().collect();
        let (clink_cells,clink_constraints)=append_clink_l9_relation(
            &mut tables,&mut resident_cells,&shared_factor_cells.iter().copied().collect::<Vec<_>>())?;
        let graph=json!({"source_word":source.as_word(),"old_to_resident":map,"boundary":boundary,
            "preparation":"native nested gates and equality transport, before DQI elimination or decisions",
            "cell_carrier":"Belnap FOUR","belnap_domains":relation.belnap_domains.iter().map(|d|d.mask()).collect::<Vec<_>>(),
            "belnap_prepared_bank":relation.belnap_prepared_bank.iter().map(|d|d.mask()).collect::<Vec<_>>(),
            "belnap_preparation":{"evidence":relation.belnap_preparation.evidence.name(),"sweeps":relation.belnap_preparation.sweeps,
                "updates":relation.belnap_preparation.updates,"held":relation.belnap_preparation.held.iter().map(|(gate,domains)|json!([gate,domains.iter().map(|d|d.mask()).collect::<Vec<_>>()])).collect::<Vec<_>>(),
                "numeral_projection":relation.belnap_preparation.numeral_projection},
            "masks":relation.masks,"p":relation.p,"q":relation.q,"midpoint":relation.midpoint,
            "gap":relation.gap,"parity":relation.parity,"admission":relation.admission,
            "gates":relation.gates.iter().map(|g|json!({"cells":g.cells,"kind":format!("{:?}",g.full_kind),
                "full_supports":g.full_supports,"full_rows":g.full_rows,
                "numeral_supports":g.supports,"numeral_rows":g.all})).collect::<Vec<_>>(),
            "sic_d12":{"dimension":12,"outcomes":144,"source_product_residue":source_residue,
                "factor_address":"(p mod 12, q mod 12) from the same factor bit cells",
                "mass_numerators":"13 at selected outcome; 1 at each other outcome; denominator 156; one-hot total 156",
                "dual":"coordinate numerator = 13*mass_i - mass_sum/12; matrix (13/12)I - (1/144)J; recovered coordinate denominator 1872",
                "shared_auxiliary_cells":sic_cells,"shared_constraints":sic_constraints},
            "clink_l9":{"catalog_source":"$HOME/.imscrbgrmr/catalog.json:clink_l9",
                "word":clink_word,"criticality_slot":clink_criticality.to_string(),
                "paired_factor_bits":shared_factor_cells.len(),"law":"FFUSE(FSPLIT(x)) = x",
                "work_opcode":"⊤ (EVALT) on each paired path","shared_auxiliary_cells":clink_cells,
                "shared_constraints":clink_constraints},
            "type_logic":{"carrier":"ResidentDialectic on the same N-bound factor domains",
                "source":type_record.source,"word":type_record.word,"boundary":type_record.boundary,
                "span":type_record.span,"midpoint_frontier":type_record.frontier,
                "verdict":type_record.verdict.to_string(),"lattice":format!("{:?}",type_record.lattice),
                "phase":type_record.phase,"evidence":type_record.evidence,
                "Y_contractions":type_record.operations.contractions,
                "Y_descents":type_record.operations.descents,
                "temporal_ticks":type_record.operations.temporal_ticks,
                "information_joins":type_record.operations.information_joins,
                "closure_pair":type_record.closure_pair}});
        for gate in &relation.gates{
            let scope:Vec<_>=gate.cells.iter().copied().filter(|&w|fixed[w].is_none()||map[w]<boundary)
                .map(|w|map[w]).collect::<BTreeSet<_>>().into_iter().collect();
            let mut rows=BTreeSet::new();
            for row in 0..gate.full_kind.table().rows {
                if gate.full_rows&(1u64<<row)==0{continue;}
                let full=gate.full_kind.row(row);
                let mut values=BTreeMap::new();let mut valid=true;
                for (at,&old) in gate.cells.iter().enumerate(){
                    if !relation.belnap_domains[old].contains(full[at]){valid=false;break;}
                    let mark=full[at].definite().ok_or("coherent numeral projection retains N/B alternatives")?;
                    if fixed[old].is_some_and(|v|v!=mark)||values.insert(map[old],mark).is_some_and(|v|v!=mark){valid=false;break;}
                }
                if valid{rows.insert(scope.iter().enumerate().fold(0usize,|r,(at,w)|r|(usize::from(values[w])<<at)));}
            }
            if scope.is_empty()&&rows.len()==1{continue;}
            tables.push((scope,rows.into_iter().collect()));
        }
        for (old,mark) in fixed.iter().enumerate(){if let Some(value)=mark{
            if map[old]<boundary{tables.push((vec![map[old]],vec![usize::from(*value)]));}
        }}
        for (at,&mark) in tape.iter().enumerate(){tables.push((vec![at],vec![usize::from(mark=='⊥')]));}
        let mut arena=DecisionArena::new(resident_cells);
        let projector=Projector::from_relations(&mut arena,boundary,tables)?;
        eprintln!("godel-complex preparation complex-lift seconds={:.6} nodes={} internal={}",
            preparation_clock.elapsed().as_secs_f64(),arena.retained_nodes(),projector.internal_count());
        let mut unit=DecisionArena::new(0);let one=unit.basis(&BigUint::zero(),BigInt::one());
        let initial_bank=arena.import_shifted(&unit,one,0)?;let zero=arena.zero();
        let source_literals:Vec<_>=tape.iter().enumerate().map(|(w,m)|(w,*m=='⊥')).collect();
        let source_bank=arena.conditional_literals(zero,initial_bank,&source_literals);
        let frame=FixedQuquartSic::new(&FixedPointFormat::for_modulus_bits(n.bits())?)?;
        eprintln!("godel-complex preparation SIC-frame seconds={:.6}",preparation_clock.elapsed().as_secs_f64());
        let mut live=vec![source_bank,initial_bank];live.extend(projector.roots());arena.fold_roots(&live);
        let rotation_base=vox_core::morphism_factor::decimal_to_tape("2").ok_or("base 2 did not encode")?;
        let rotation_modulus=vox_core::morphism_factor::parse_numeral(&source.as_word()).map_err(|e|e.to_string())?;
        let rotation_residue=vox_core::morphism_factor::modulo(&rotation_base,&rotation_modulus);
        let mut carrier=Self{source,arena,projector:Some(projector),source_bank,initial_bank,frame_banks:Vec::new(),rotation_base,rotation_modulus,rotation_residue,
            rotation_order:vox_core::morphism_factor::one(),rotation_closed:false,p,q,boundary,
            graph,catalyst,frame,steps:0,exchanges:Vec::new(),certificate_pool:BTreeMap::new(),chart_cache:BTreeMap::new(),closed:None,resolved:None};
        carrier.attach_native_frame()?;Ok(carrier)
    }
    /// Execute the recovered SHIAB residual sweep on this carrier's own N.
    /// A proper divisor returned by that instrument constrains these existing
    /// factor cells; an open frame remains as five retained observation banks.
    pub fn attach_native_frame(&mut self)->Result<(),String>{
        if !self.frame_banks.is_empty(){return Ok(());}
        let n=self.source.to_biguint();
        let native=vox_core::godel_calculus::Nat::from_decimal(&n.to_string()).ok_or("invalid native frame source")?;
        let f=vox_core::godel_product::residual_factor_frame(&native).ok_or("native source frame missing")?;
        let selected=vox_core::godel_product::residual_frame_factor(&native,&f);
        let selected_label=selected.as_ref().map(|(label,_)|*label);
        let candidate=selected.as_ref().map(|(_,value)|value.clone()).unwrap_or_else(||f.common.clone());
        let values=[f.a,f.b,f.product,f.residual_magnitude,candidate];
        let words=values.iter().map(|v|WordTape::from_biguint(&v.to_string().parse::<BigUint>().unwrap())).collect::<Vec<_>>();
        let zero=self.arena.zero();
        for word in &words{
            let bits=word.parity_cells();let pins:Vec<_>=(0..self.source.bit_len()).map(|w|(w,bits.get(w)==Some(&'⊥'))).collect();
            self.frame_banks.push(self.arena.conditional_literals(zero,self.initial_bank,&pins));
        }
        let d=words[4].to_biguint();let proper=selected.is_some()&&d>BigUint::one()&&d<n&&(&n%&d).is_zero();
        self.graph["residual_frame"]=json!({"shift":f.shift,"shifts_tested":f.shifts_tested,
            "words":words.iter().map(WordTape::as_word).collect::<Vec<_>>(),
            "order":["even_lane","odd_lane","lane_product","residual","selected_gcd"],
            "residual_gcd":f.common.to_string(),"even_lane_gcd":f.a_common.to_string(),
            "odd_lane_gcd":f.b_common.to_string(),"selected_gcd":selected_label,
            "proper_divisor":proper});
        if proper{
            let q=&n/&d;let (p,q)=if d<=q{(d,q)}else{(q,d)};
            let mut pins=BTreeMap::new();
            for (cells,value) in assign_factor_lanes(&self.p,&self.q,&p,&q)?{for (at,&cell) in cells.iter().enumerate(){
                let bit=value.bit(at as u64);if pins.insert(cell,bit).is_some_and(|v|v!=bit){return Err("native frame contradicted its shared factor cells".into());}
            }}
            if let Some(projector)=self.projector.as_mut(){projector.retain_boundary_literals(&mut self.arena,&pins.into_iter().collect::<Vec<_>>())?;}
        }
        Ok(())
    }
    fn sic_chart(&mut self,root:usize,scope:&[usize])->Result<String,String>{
        self.sic_chart_focused(root,scope,None)
    }
    fn sic_chart_focused(&mut self,root:usize,scope:&[usize],focus:Option<usize>)->Result<String,String>{
        self.sic_chart_ordered(root,scope,&focus.into_iter().collect::<Vec<_>>())
    }
    fn sic_chart_ordered(&mut self,root:usize,scope:&[usize],focus:&[usize])->Result<String,String>{
        let signature=format!("{:?}:{}",focus.iter().map(|w|scope.binary_search(w)).collect::<Vec<_>>(),self.arena.scope_signature(root,scope)?);
        if let Some(key)=self.chart_cache.get(&signature){return Ok(key.clone());}
        let (mut local,mut root)=self.arena.scoped(root,scope)?;
        if !focus.is_empty(){
            let mut order=focus.iter().map(|wire|scope.binary_search(wire)
                .map_err(|_|"working-stroke wire escaped its frame".to_string())).collect::<Result<Vec<_>,_>>()?;
            order.extend((0..scope.len()).filter(|w|!focus.contains(&scope[*w])));
            let mut map=vec![0;scope.len()];for (new,&old) in order.iter().enumerate(){map[old]=new;}
            let (renamed,next)=local.permuted(root,&map)?;local=renamed;root=next;
        }
        // A scalar or unary tensor is embedded with explicit zero spectators.
        let original=local.cells;local.cells=local.cells.max(2);let zero=local.zero();
        let padding:Vec<_>=(original..local.cells).map(|w|(w,false)).collect();
        let root=local.conditional_literals(zero,root,&padding);
        let mut arms=[zero;4];
        for (label,arm) in arms.iter_mut().enumerate(){
            let controls=[(0,label&1!=0),(1,label&2!=0)];
            *arm=local.conditional_literals(zero,root,&controls);
            for (w,b) in controls{if b{*arm=local.flip(*arm,w);}}
        }
        let gram=local.control_gram(&arms);
        let key=gram.iter().map(|(r,i)|format!("{r},{i}")).collect::<Vec<_>>().join(";");
        if !self.certificate_pool.get(&key).is_some_and(|v|v.get("recovered_gram").is_some()){
            let masses=self.frame.gram_mass_numerators(&gram)?;
            let recovered=self.frame.certify_gram_frame_unrounded_masses(&gram,&masses)?;
            self.certificate_pool.insert(key.clone(),json!({
                "gram":gram.iter().map(|(r,i)|(r.to_string(),i.to_string())).collect::<Vec<_>>(),
                "mass_numerators":masses.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "recovered_gram":recovered.rounded.recovered.iter().map(|(r,i)|(r.to_string(),i.to_string())).collect::<Vec<_>>(),
                "denominator":recovered.denominator.to_string(),
                "signed_rational_residuals":recovered.signed_residual_numerators.iter().map(|(r,i)|(r.to_string(),i.to_string())).collect::<Vec<_>>(),
                "maximum_residual":recovered.rounded.maximum_residual.to_string(),
                "tolerance":recovered.rounded.tolerance.to_string()}));
        }
        self.chart_cache.insert(signature,key.clone());Ok(key)
    }
    /// The full complex SIC dual now drives a support return on the same
    /// incident family. Disagreement is retained as B evidence and the union
    /// of its supported labels is held; it never licenses an exclusion.
    fn return_joint_sic_support(&mut self,wire:usize)->Result<(),String>{
        loop{
            let projector=self.projector.as_ref().ok_or("missing SIC return carrier")?;
            if !projector.carries_internal(wire){break;}
            let (joint,scope)=projector.joint_boundary_arm(&mut self.arena,wire)?;
            let mut focus=scope.iter().copied().filter(|w|*w<self.boundary&&
                (self.p.contains(w)||self.q.contains(w))).take(2).collect::<Vec<_>>();
            if focus.len()<2&&!focus.contains(&wire){focus.push(wire);}
            if focus.len()<2{if let Some(&other)=scope.iter().find(|w|!focus.contains(w)){focus.push(other);}}
            focus.sort_unstable();
            let key=self.sic_chart_ordered(joint,&scope,&focus)?;
            let certificate=&self.certificate_pool[&key];
            let mut observed=0u8;let mut exact=0u8;
            for label in 0..1usize<<focus.len(){
                let diagonal=5*label;
                let read=|name:&str,part:usize|certificate[name][diagonal][part].as_str()
                    .ok_or("lost shared SIC coordinate")?.parse::<BigInt>().map_err(|_|"invalid shared SIC coordinate");
                let recovered_re=read("recovered_gram",0)?;let recovered_im=read("recovered_gram",1)?;
                if !recovered_re.is_zero()||!recovered_im.is_zero(){observed|=1<<label;}
                if !read("gram",0)?.is_zero()||!read("gram",1)?.is_zero(){exact|=1<<label;}
            }
            let supported=observed|exact;
            let paradox=observed!=exact||supported==0;
            let changed=if supported==0{0}else{self.projector.as_mut().unwrap()
                .return_measured_support(&mut self.arena,&focus,supported)?};
            let ledger=self.graph.as_object_mut().ok_or("lost shared graph")?
                .entry("sic_support_return").or_insert_with(||json!({
                    "format":"sic-shared-support-return-v1","measurements":0,"changed_tensors":0,
                    "B_observations":0,"fixed_points":0,"ancestry":[],
                    "work":"full complex dual -> shared support projection -> incident-arm re-entry -> fixed point -> same-wire fuse"}));
            for (name,delta) in [("measurements",1u64),("changed_tensors",changed as u64),
                ("B_observations",u64::from(paradox)),("fixed_points",u64::from(changed==0))]{
                ledger[name]=json!(ledger[name].as_u64().unwrap_or(0)+delta);
            }
            let observation=json!({"wire":wire,"focus":focus,"scope":scope,"certificate":key,
                "measured_labels":observed,"exact_labels":exact,"held_labels":supported,
                "verdict":if paradox{"B"}else if changed==0{"N"}else{"T"},
                "changed_tensors":changed,"fixed_point":changed==0});
            if changed!=0||paradox{ledger["ancestry"].as_array_mut().unwrap().push(observation.clone());}
            ledger["last"]=observation;
            if changed==0{break;}
        }
        Ok(())
    }
    pub fn advance(&mut self)->Result<bool,String>{
        if self.closed.is_some(){return Ok(true);}
        if !self.rotation_closed {
            let returned=advance_relative_rotation(&self.rotation_base,&self.rotation_modulus,&mut self.rotation_residue,&mut self.rotation_order);
            if self.graph.get("rotation_opening").is_none(){
                if let Some((opening,factor,quotient))=vox_core::shor_braid::factor_close_at_residue(&self.rotation_residue,&self.rotation_modulus)?{
                    self.bind_rotation_factor(&decode_tape(&factor),&decode_tape(&quotient),opening)?;
                }
            }
            if returned {
                self.rotation_closed=true;
                let closed=vox_core::shor_braid::factor_close_public(&self.rotation_base,&self.rotation_modulus,&self.rotation_order);
                if let Ok((left,right))=closed {
                    let to_big=|tape:&[char]|{let mut bytes=vec![0u8;(tape.len()+7)/8];for (i,m) in tape.iter().enumerate(){if *m=='⊥'{bytes[i/8]|=1<<(i%8);}}BigUint::from_bytes_le(&bytes)};
                    let a=to_big(&left);let b=to_big(&right);let n=self.source.to_biguint();
                    if a>BigUint::one()&&b>BigUint::one()&&&a*&b==n {
                        self.bind_rotation_factor(&a,&b,"full-order-close")?;
                    }
                }
            }
        }
        if self.steps<2{eprintln!("godel-complex step={} enter",self.steps);}
        let turn=validate_cycle(&self.catalyst)?;
        let exchange=self.projector.as_mut().unwrap().exchange_at_boundary(&mut self.arena,turn)?;
        if let Some((wire,a,b))=exchange{
            // Recover every arm changed by the numerical working stroke before
            // its fuse. Bank indices are recorded separately from compact lists.
            let p=self.projector.as_ref().unwrap();
            let arms=[p.enclosing_arm(a)?,p.enclosing_arm(b)?];
            for (root,scope) in &arms{self.sic_chart_focused(*root,scope,Some(wire))?;}
            // Scopes survive bank compaction on restoration; ephemeral vector
            // indices are not persistent ancestry identifiers.
            self.exchanges.push((wire,arms[0].1.clone(),arms[1].1.clone()));
            self.return_joint_sic_support(wire)?;
        }
        let done=if let Some((wire,_,_))=exchange{
            self.projector.as_mut().unwrap().advance_at_boundary(&mut self.arena,wire)?
        }else{self.projector.as_mut().unwrap().advance(&mut self.arena)?};self.steps+=1;
        if self.steps<=2{eprintln!("godel-complex step={} fused",self.steps);}
        if done{
            let projector=self.projector.take().unwrap();
            // The unit bank has no dependencies; the source bank is retained.
            let mut banks=vec![self.source_bank,self.initial_bank];banks.extend(&self.frame_banks);
            let root=projector.finish_banked(&mut self.arena,&banks)?;
            self.closed=Some(root);
            let scope:Vec<_>=(0..self.boundary).collect();self.sic_chart(root,&scope)?;
            // Numerical extraction and prime resolution belong to the active
            // transformation. The terminal reader only returns these words.
            self.resolve_prime_boundary()?;
        }
        let mut retained=vec![self.source_bank,self.initial_bank];
        retained.extend(&self.frame_banks);
        if let Some(p)=&self.projector{retained.extend(p.roots());}
        if let Some(root)=self.closed{retained.push(root);}
        self.arena.fold_roots(&retained);
        Ok(done)
    }
    fn bind_rotation_factor(&mut self,factor:&BigUint,quotient:&BigUint,opening:&str)->Result<(),String>{
        let n=self.source.to_biguint();
        if factor<=&BigUint::one()||factor>=&n||factor*quotient!=n{return Err("rotation opening failed exact source factor closure".into());}
        let mut pins=BTreeMap::new();
        for (cells,value) in assign_factor_lanes(&self.p,&self.q,factor,quotient)?{for(at,&cell)in cells.iter().enumerate(){
            if pins.insert(cell,value.bit(at as u64)).is_some_and(|old|old!=value.bit(at as u64)){return Err("rotation opening contradicted shared factor cells".into());}
        }}
        if let Some(projector)=self.projector.as_mut(){projector.retain_boundary_literals(&mut self.arena,&pins.into_iter().collect::<Vec<_>>())?;}
        self.graph["rotation_opening"]=json!({"coordinate_order":vox_core::morphism_factor::dec_of(&self.rotation_order),
            "residue":vox_core::morphism_factor::dec_of(&self.rotation_residue),"opening":opening,
            "factor":factor.to_string(),"quotient":quotient.to_string(),"product_verified":true,
            "boundary":"after residue read, before this step's Frobenius FSPLIT/FFUSE contraction"});
        println!("rotation.open coordinate={} residue={} witness={} factor={} quotient={} product={} verified=true before=Frobenius-fuse",
            vox_core::morphism_factor::dec_of(&self.rotation_order),
            vox_core::morphism_factor::dec_of(&self.rotation_residue),opening,factor,quotient,n);
        Ok(())
    }
    pub fn progress(&self)->Value{json!({"steps":self.steps,"rotation_residue":vox_core::morphism_factor::dec_of(&self.rotation_residue),"rotation_order":vox_core::morphism_factor::dec_of(&self.rotation_order),"rotation_closed":self.rotation_closed,"nodes":self.arena.retained_nodes(),
        "internal":self.projector.as_ref().map_or(0,|p|p.internal_count()),
        "contractions":self.projector.as_ref().map_or(self.steps,|p|p.contractions),
        "exchanges":self.exchanges.len(),"closed":self.closed.is_some(),"prime_resolved":self.resolved.is_some(),"SIC_frames":self.certificate_pool.len(),
        "SIC_support_measurements":self.graph["sic_support_return"]["measurements"],
        "SIC_changed_tensors":self.graph["sic_support_return"]["changed_tensors"],
        "SIC_B_observations":self.graph["sic_support_return"]["B_observations"],
        "support_fixed_points":self.graph["sic_support_return"]["fixed_points"]})}
    pub fn held_record(&mut self)->Result<Value,String>{
        let banks=self.projector.as_ref().map_or_else(Vec::new,|p|p.enclosing_banks());
        let mut ancestry=Vec::new();
        let bank_count=banks.len();
        for (at,(root,scope)) in banks.into_iter().enumerate(){
            let certificate=self.sic_chart(root,&scope)?;ancestry.push(json!({"scope":scope,"certificate":certificate}));
            if at>0&&at%16384==0{eprintln!("godel-complex recover banks={at}/{bank_count} signatures={}",self.certificate_pool.len());}
        }
        let source_scope:Vec<_>=(0..self.source.bit_len()).collect();
        let source_certificate=self.sic_chart(self.source_bank,&source_scope)?;
        let initial_certificate=self.sic_chart(self.initial_bank,&[])?;
        let mut frame_certificates=Vec::new();
        for bank in self.frame_banks.clone(){frame_certificates.push(self.sic_chart(bank,&source_scope)?);}
        let closed_certificate=if let Some(root)=self.closed{
            Some(self.sic_chart(root,&(0..self.boundary).collect::<Vec<_>>())?)
        }else{None};
        let mut roots=vec![self.source_bank,self.initial_bank];
        roots.extend(&self.frame_banks);
        if let Some(root)=self.closed{roots.push(root);}
        if let Some(p)=&self.projector{roots.extend(p.roots());}
        Ok(json!({"format":"godel-complex-carrier-v1","source_word":self.source.as_word(),
            "graph":self.graph,"catalyst":self.catalyst,"steps":self.steps,"exchanges":self.exchanges,
            "relative_rotation":{"base":vox_core::morphism_factor::dec_of(&self.rotation_base),"modulus":vox_core::morphism_factor::dec_of(&self.rotation_modulus),"residue":vox_core::morphism_factor::dec_of(&self.rotation_residue),"order":vox_core::morphism_factor::dec_of(&self.rotation_order),"closed":self.rotation_closed},
            "p":self.p,"q":self.q,"boundary":self.boundary,"closed":self.closed.is_some(),
            "frame_bank_count":self.frame_banks.len(),
            "resolved_words":self.resolved.as_ref().map(|(p,q)|[p.as_word(),q.as_word()]),
            "register":self.arena.compiled_record_many(&roots),
            "projector":self.projector.as_ref().map(|p|p.preparation_record()),
            "SIC_frame":self.frame.prepared_record(),"SIC_pool":self.certificate_pool,
            "SIC_bank_ancestry":ancestry,"progress":self.progress(),
            "SIC_source_bank":source_certificate,"SIC_initial_bank":initial_certificate,
            "SIC_residual_frame_banks":frame_certificates,
            "SIC_closed_factor_bank":closed_certificate,
            "lowering":"executed winding delta -> inverse Gaussian gauges on shared arms; FSPLIT/FFUSE contracts their same wire",
            "SIC_scope":"full complex ququart Gram per enclosing tensor bank; global factor-state Gram only after complete contraction"}))
    }
    pub fn from_held_record(v:&Value)->Result<Self,String>{
        if v["format"]!="godel-complex-carrier-v1"{return Err("unknown Gödel carrier format".into());}
        validate_cycle(&v["catalyst"])?;
        let source=WordTape::from_canonical_word(v["source_word"].as_str().ok_or("missing source bank")?).ok_or("invalid source bank")?;
        if v["graph"]["source_word"]!=v["source_word"]{return Err("source graph changed its source".into());}
        let (arena,roots)=DecisionArena::from_prepared_record_many(&v["register"])?;
        let closed=v["closed"].as_bool().ok_or("missing closed boundary")?;
        let frame_count=v["frame_bank_count"].as_u64().unwrap_or(0) as usize;
        if frame_count!=0&&frame_count!=5{return Err("incomplete residual frame banks".into());}
        let offset=2+frame_count;
        if roots.len()<offset+usize::from(closed){return Err("lost enclosing source banks".into());}
        let p=indices(v,"p")?;let q=indices(v,"q")?;let boundary=integer(v,"boundary")?;
        if boundary>arena.cells||p.iter().chain(&q).any(|&w|w>=boundary){return Err("factor map escaped its boundary".into());}
        let projector=if closed{None}else{Some(Projector::from_preparation_record(&v["projector"],&roots[offset..])?)};
        let frame=FixedQuquartSic::from_prepared_record(&v["SIC_frame"])?;
        let rotation=&v["relative_rotation"];
        let parse_rotation=|key:&str|vox_core::morphism_factor::decimal_to_tape(rotation[key].as_str().ok_or("missing relative rotation coordinate")?).ok_or_else(||"invalid relative rotation coordinate".to_string());
        let rotation_base=parse_rotation("base")?;let rotation_modulus=parse_rotation("modulus")?;let rotation_residue=parse_rotation("residue")?;let rotation_order=parse_rotation("order")?;
        let rotation_closed=rotation["closed"].as_bool().ok_or("missing relative rotation closure")?;
        if vox_core::morphism_factor::cmp(&rotation_modulus,&vox_core::morphism_factor::parse_numeral(&source.as_word()).map_err(|e|e.to_string())?)!=core::cmp::Ordering::Equal{return Err("held rotation modulus changed its source".into());}
        if vox_core::morphism_factor::dec_of(&rotation_base)!="2"{return Err("held relative rotation changed fixed base 2".into());}
        let mut expected=vox_core::morphism_factor::one();let mut power=rotation_base.clone();
        for (at,bit) in rotation_order.iter().enumerate(){
            if *bit=='⊥'{expected=vox_core::morphism_factor::modulo(&vox_core::morphism_factor::mul(&expected,&power),&rotation_modulus);}
            if at+1<rotation_order.len(){power=vox_core::morphism_factor::modulo(&vox_core::morphism_factor::mul(&power,&power),&rotation_modulus);}
        }
        if expected!=rotation_residue||rotation_closed!=(vox_core::morphism_factor::cmp(&rotation_residue,&vox_core::morphism_factor::one())==core::cmp::Ordering::Equal){return Err("held relative rotation failed per-coordinate order validation".into());}
        let mut result=Self{source,arena,projector,source_bank:roots[0],initial_bank:roots[1],frame_banks:roots[2..offset].to_vec(),rotation_base,rotation_modulus,rotation_residue,rotation_order,rotation_closed,p,q,boundary,
            graph:v["graph"].clone(),catalyst:v["catalyst"].clone(),frame,steps:integer(v,"steps")?,
            exchanges:serde_json::from_value(v["exchanges"].clone()).map_err(|e|e.to_string())?,
            certificate_pool:serde_json::from_value(v["SIC_pool"].clone()).map_err(|e|e.to_string())?,
            chart_cache:BTreeMap::new(),
            closed:if closed{Some(roots[offset])}else{None},resolved:None};
        let mut unit=DecisionArena::new(0);let one=unit.basis(&BigUint::zero(),BigInt::one());
        let one=result.arena.import_shifted(&unit,one,0)?;
        if one!=result.initial_bank{return Err("restored prepared unit bank changed".into());}
        let tape=result.source.parity_cells();let zero=result.arena.zero();
        let literals:Vec<_>=tape.iter().enumerate().map(|(w,m)|(w,*m=='⊥')).collect();
        let expected_source=result.arena.conditional_literals(zero,one,&literals);
        if expected_source!=result.source_bank{return Err("restored source bank changed its numerical source".into());}
        for (at,&bank) in result.frame_banks.iter().enumerate(){
            let word=WordTape::from_canonical_word(result.graph["residual_frame"]["words"][at].as_str().ok_or("lost residual frame word")?).ok_or("invalid residual frame word")?;
            let bits=word.parity_cells();let pins:Vec<_>=(0..tape.len()).map(|w|(w,bits.get(w)==Some(&'⊥'))).collect();
            if result.arena.conditional_literals(zero,one,&pins)!=bank{return Err("restored residual frame bank changed".into());}
        }
        if let Some(p)=&result.projector{p.check_carrier(result.arena.cells,&(0..tape.len()).collect::<Vec<_>>())?;}
        let map=indices(&result.graph,"old_to_resident")?;
        for (name,actual) in [("p",&result.p),("q",&result.q)]{
            let old=indices(&result.graph,name)?;
            let expected=old.iter().map(|&w|map.get(w).copied().ok_or("held graph lost its factor address"))
                .collect::<Result<Vec<_>,_>>()?;
            if &expected!=actual{return Err("held factor map changed its ancestral cells".into());}
        }
        // Validate the complete measurement ledger, including pre-fuse complex
        // arms that no longer occur in the current contracted bank list.
        for (key,certificate) in &result.certificate_pool{
            let pairs:Vec<(String,String)>=serde_json::from_value(certificate["gram"].clone()).map_err(|e|e.to_string())?;
            let parsed=pairs.iter().map(|(r,i)|Ok((r.parse::<BigInt>().map_err(|_|"invalid signed Gram")?,
                i.parse::<BigInt>().map_err(|_|"invalid signed Gram")?))).collect::<Result<Vec<_>,String>>()?;
            let gram:[(BigInt,BigInt);16]=parsed.try_into().map_err(|_|"incomplete full complex Gram")?;
            let expected_key=gram.iter().map(|(r,i)|format!("{r},{i}")).collect::<Vec<_>>().join(";");
            if key!=&expected_key{return Err("held SIC frame changed its key".into());}
            let strings:Vec<String>=serde_json::from_value(certificate["mass_numerators"].clone()).map_err(|e|e.to_string())?;
            let masses:[BigUint;16]=strings.iter().map(|m|m.parse::<BigUint>().map_err(|_|String::from("invalid projector mass")))
                .collect::<Result<Vec<_>,_>>()?.try_into().map_err(|_|"incomplete projector masses")?;
            if masses!=result.frame.gram_mass_numerators(&gram)?{return Err("held masses changed their measured frame".into());}
            let recovered=result.frame.certify_gram_frame_unrounded_masses(&gram,&masses)?;
            let residuals=json!(recovered.signed_residual_numerators.iter().map(|(r,i)|(r.to_string(),i.to_string())).collect::<Vec<_>>());
            if certificate["signed_rational_residuals"]!=residuals
                ||certificate["denominator"]!=recovered.denominator.to_string()
                ||certificate["maximum_residual"]!=recovered.rounded.maximum_residual.to_string()
                ||certificate["tolerance"]!=recovered.rounded.tolerance.to_string(){return Err("held signed SIC residuals changed".into());}
        }
        // Recompute all current bank measurements, rather than trust saved
        // certificates as a substitute for restored amplitude validation.
        let expected=v["SIC_bank_ancestry"].as_array().ok_or("missing held SIC ancestry")?;
        let banks=result.projector.as_ref().map_or_else(Vec::new,|p|p.enclosing_banks());
        if expected.len()!=banks.len(){return Err("restored SIC bank count changed".into());}
        for (at,((root,scope),saved)) in banks.into_iter().zip(expected).enumerate(){
            let saved_scope:Vec<usize>=serde_json::from_value(saved["scope"].clone()).map_err(|e|e.to_string())?;
            if scope!=saved_scope||saved["certificate"]!=result.sic_chart(root,&scope)?{
                return Err("restored SIC ancestry changed".into());
            }
            if at>0&&at%16384==0{eprintln!("godel-complex restore SIC banks={at}/{}",expected.len());}
        }
        let source_scope:Vec<_>=(0..tape.len()).collect();
        if v["SIC_source_bank"]!=result.sic_chart(result.source_bank,&source_scope)?
            ||v["SIC_initial_bank"]!=result.sic_chart(result.initial_bank,&[])?{
            return Err("restored SIC source ancestry changed".into());
        }
        if frame_count!=0{
            let mut certificates=Vec::new();
            for root in result.frame_banks.clone(){certificates.push(result.sic_chart(root,&source_scope)?);}
            if v["SIC_residual_frame_banks"]!=json!(certificates){return Err("restored frame SIC ancestry changed".into());}
        }
        let closed_certificate=if let Some(root)=result.closed{
            Some(result.sic_chart(root,&(0..result.boundary).collect::<Vec<_>>())?)
        }else{None};
        if v.get("SIC_closed_factor_bank").is_some()&&v["SIC_closed_factor_bank"]!=json!(closed_certificate){
            return Err("restored global factor-state SIC ancestry changed".into());
        }
        if closed{
            result.resolve_prime_boundary()?;
            if let Some(words)=v.get("resolved_words").filter(|v|!v.is_null()){
                let actual=json!(result.resolved.as_ref().map(|(p,q)|[p.as_word(),q.as_word()]));
                if &actual!=words{return Err("restored prime resolution changed".into());}
            }
        }
        Ok(result)
    }
    fn resolve_prime_boundary(&mut self)->Result<(),String>{
        let Some(root)=self.closed else{return Ok(());};
        let sample=self.arena.sample_prefix(root,self.boundary,&BigUint::zero(),&BigUint::one())?;
        let decode=|wires:&[usize]|wires.iter().enumerate().fold(BigUint::zero(),|mut n,(at,&wire)|{
            if sample.bit(wire as u64){n.set_bit(at as u64,true);}n});
        let p=decode(&self.p);let q=decode(&self.q);let n=self.source.to_biguint();
        if p<=BigUint::one()||q<=BigUint::one()||&p*&q!=n{return Err("complex return failed exact product closure".into());}
        if !(&n%&p).is_zero()||&n/&p!=q{return Err("derived quotient failed the source boundary".into());}
        let p=WordTape::from_biguint(&p);let q=WordTape::from_biguint(&q);
        let closure=vox_core::morphism_factor::verify(p.as_word(),q.as_word(),self.source.as_word())?;
        if !closure.starts_with("p*q == N: true\n"){return Err("emitted factor words failed IMASM product closure".into());}
        if vox_core::morphism_factor::miller_rabin(&p.parity_cells())&&vox_core::morphism_factor::miller_rabin(&q.parity_cells()){
            self.resolved=Some((p,q));
        }
        Ok(())
    }
    pub fn readout(&self)->Result<Option<(BigUint,BigUint)>,String>{
        Ok(self.resolved.as_ref().map(|(p,q)|(p.to_biguint(),q.to_biguint())))
    }
    fn released_rotation_pair(&self)->Result<Option<(BigUint,BigUint)>,String>{
        let Some(opening)=self.graph.get("rotation_opening") else{return Ok(None);};
        let parse=|key:&str|opening[key].as_str().and_then(|s|BigUint::parse_bytes(s.as_bytes(),10)).ok_or_else(||format!("invalid released rotation {key}"));
        let factor=parse("factor")?;let quotient=parse("quotient")?;
        if factor<=BigUint::one()||factor>=self.source.to_biguint()||&factor*&quotient!=self.source.to_biguint(){
            return Err("released rotation pair failed exact source closure".into());
        }
        Ok(Some((factor,quotient)))
    }
}

/// Opt-in integrated factor entry. The source-only preparation and the held
/// return are explicit; resume consumes the returned register without preparing
/// another graph. This entry neither invokes the old bit-decision producer nor
/// accepts supplied factors.
pub fn command(args:&[&str])->Result<String,String>{
    use std::time::Instant;
    let started=Instant::now();let at;
    let mut carrier=if args.first()==Some(&"--resume"){
        let path=args.get(1).ok_or("factor --resume RETURN --hold STEPS NEXT")?;at=2;
        GodelComplexCarrier::from_held_record(&serde_json::from_slice(&std::fs::read(path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?)?
    }else{
        let n=args.first().ok_or("factor N --cycle CYCLE --hold STEPS RETURN")?;
        if args.get(1)!=Some(&"--cycle"){return Err("factor N --cycle CYCLE --hold STEPS RETURN".into());}
        let path=args.get(2).ok_or("missing executed MoDoT cycle")?;at=3;
        GodelComplexCarrier::prepare(n,serde_json::from_slice(&std::fs::read(path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?)?
    };
    carrier.attach_native_frame()?;
    let preparation=started.elapsed().as_secs_f64();
    if args.get(at)!=Some(&"--hold"){return Err("integrated factor entry requires --hold STEPS RETURN".into());}
    let steps:usize=args.get(at+1).ok_or("missing held step count")?.parse().map_err(|_|"invalid held step count")?;
    let path=args.get(at+2).ok_or("missing held return path")?;
    if args.len()!=at+3{return Err("unexpected integrated factor arguments".into());}
    eprintln!("godel-complex prepared seconds={preparation:.6} {}",carrier.progress());
    let transformation=Instant::now();let mut last=Instant::now();
    let seconds=std::env::var("GODEL_TRANSFORM_SECONDS").ok().map(|v|v.parse::<f64>())
        .transpose().map_err(|_|"invalid transformation time boundary")?.unwrap_or(300.0);
    if !seconds.is_finite()||seconds<=0.0{return Err("invalid transformation time boundary".into());}
    for _ in 0..steps{
        if transformation.elapsed().as_secs_f64()>=seconds{break;}
        if carrier.advance()?{break;}
        if last.elapsed().as_secs()>=5{
            eprintln!("godel-complex transform seconds={:.6} {}",transformation.elapsed().as_secs_f64(),carrier.progress());last=Instant::now();
        }
    }
    let transform=transformation.elapsed().as_secs_f64();let returning=Instant::now();
    let mut record=carrier.held_record()?;
    record["clocks"]=json!({"prepare_or_restore_seconds":preparation,"transformation_seconds":transform});
    let bytes=serde_json::to_vec(&record).map_err(|e|e.to_string())?;
    let path=std::path::Path::new(path);let pending=path.with_extension("json.pending");
    std::fs::write(&pending,bytes).map_err(|e|e.to_string())?;
    std::fs::rename(&pending,path).map_err(|e|e.to_string())?;
    let return_seconds=returning.elapsed().as_secs_f64();let read=Instant::now();
    let result=carrier.readout()?;
    let read_seconds=read.elapsed().as_secs_f64();
    let mut report=format!("source {}\ncarrier.return {}\nprogress {}\nprepare_or_restore.seconds {preparation:.9}\ntransformation.seconds {transform:.9}\nserialize_return.seconds {return_seconds:.9}\nreadout.seconds {read_seconds:.9}\n",carrier.source.to_biguint(),path.display(),carrier.progress());
    if let Some((p,q))=result{
        let n=carrier.source.to_biguint().to_string();let ps=p.to_string();let qs=q.to_string();
        report.push_str(&vox_core::godel_product::command(&["product",&n,&ps,&qs])?);
        report.push_str(&format!("emitted.factor {p}\nemitted.quotient {q}\nexact.product PASS\n"));
        let (p_word,q_word)=carrier.resolved.as_ref().ok_or("missing transformed prime words")?;
        report.push_str(&format!("emitted.factor.word {}\nemitted.quotient.word {}\nprime.screen PASS\nquotient.remainder 0\nIMASM.product PASS\n",p_word.as_word(),q_word.as_word()));
    }else if let Some((p,q))=carrier.released_rotation_pair()?{
        let n=carrier.source.to_biguint().to_string();let ps=p.to_string();let qs=q.to_string();
        report.push_str(&vox_core::godel_product::command(&["product",&n,&ps,&qs])?);
        report.push_str(&format!("rotation.factor.release {p}\nrotation.quotient.release {q}\nrotation.product PASS\nrotation.release.stage before Frobenius fuse\n"));
    }else{report.push_str("factor.return OPEN\n");}
    Ok(report)
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn sic_joint_return_preserves_complex_product_and_source_bank(){
        // This source-sized instrument checks relational support transport,
        // independently of factor discovery. No reference divisor is supplied.
        let n=BigUint::parse_bytes(b"4479106148067194240380817417142593728490011937516752269508037",10).unwrap();
        let width=n.bits() as usize;assert!(width>=200);
        let source=WordTape::from_biguint(&n);
        let (x,z,y)=(width,width+1,width+2);
        let mut arena=DecisionArena::new(width+3);
        let mut projector=Projector::from_relations(&mut arena,y,vec![
            (vec![x,y],vec![0,3]),(vec![z,y],vec![0,3]),
            (vec![x,z],vec![1,2,3]),
        ]).unwrap();
        let before=projector.roots();
        let product=arena.amplitude_product(before[0],before[1]);
        let product=arena.amplitude_product(product,before[2]);
        let expected=arena.contract_internal_wire(product,y);
        let mut unit=DecisionArena::new(0);let one=unit.basis(&BigUint::zero(),BigInt::one());
        let one=arena.import_shifted(&unit,one,0).unwrap();let zero=arena.zero();
        let pins:Vec<_>=(0..width).map(|w|(w,n.bit(w as u64))).collect();
        let bank=arena.conditional_literals(zero,one,&pins);
        assert!(projector.exchange_at_boundary(&mut arena,1).unwrap().is_some());
        let frame=FixedQuquartSic::new(&FixedPointFormat::for_modulus_bits(n.bits()).unwrap()).unwrap();
        let mut carrier=GodelComplexCarrier{source,arena,projector:Some(projector),
            source_bank:bank,initial_bank:one,frame_banks:vec![],
            rotation_base:vec![],rotation_modulus:vec![],rotation_residue:vec![],rotation_order:vec![],rotation_closed:false,
            p:vec![x],q:vec![z],boundary:y,graph:json!({}),catalyst:json!({}),frame,
            steps:0,exchanges:vec![],certificate_pool:BTreeMap::new(),chart_cache:BTreeMap::new(),closed:None,resolved:None};
        carrier.return_joint_sic_support(y).unwrap();
        let ledger=&carrier.graph["sic_support_return"];
        assert!(ledger["changed_tensors"].as_u64().unwrap()>0,"joint exclusion never reached its neighboring arm");
        assert_eq!(ledger["B_observations"],0);
        assert!(carrier.projector.as_mut().unwrap().advance_at_boundary(&mut carrier.arena,y).unwrap());
        let root=carrier.projector.take().unwrap().finish_banked(&mut carrier.arena,&[bank]).unwrap();
        assert_eq!(root,expected,"SIC return changed a surviving complex coefficient");
        let restored_bank=carrier.arena.conditional_literals(zero,one,&pins);
        assert_eq!(bank,restored_bank,"source spectator changed during measured return");
        assert!(carrier.certificate_pool.values().all(|v|v.get("recovered_gram").is_some()));
    }
    #[test]
    fn relative_rotation_reads_one_residue_per_step_on_hard_source(){
        let n=include_str!("../../Vox/measurements/base_mod_membrane_20261022/n1443886668465785239555226711752594674554632143872728785072260425128493389058547185473075869/source_n_only.txt").trim();
        let base=vox_core::morphism_factor::decimal_to_tape("2").unwrap();
        let modulus=vox_core::morphism_factor::decimal_to_tape(n).unwrap();
        let mut residue=vox_core::morphism_factor::modulo(&base,&modulus);
        let mut order=vox_core::morphism_factor::one();
        for exponent in 2..=5{
            assert!(!advance_relative_rotation(&base,&modulus,&mut residue,&mut order));
            assert_eq!(vox_core::morphism_factor::dec_of(&order),exponent.to_string());
            let expected=vox_core::morphism_factor::modulo(&vox_core::morphism_factor::decimal_to_tape(&2u8.pow(exponent as u32).to_string()).unwrap(),&modulus);
            assert_eq!(residue,expected);
        }
    }
    #[test]
    fn integrated_source_202_retains_complex_cycle_and_continuation(){
        let cycle:Value=serde_json::from_str(include_str!("../../measurements/prime_well_alchemy_20261022/integrated/cycle.json")).unwrap();
        let source=include_str!("../measurements/held_return_20261022/new_balanced/source.txt").trim();
        let mut direct=GodelComplexCarrier::prepare(source,cycle).unwrap();
        assert_eq!(direct.frame_banks.len(),5);
        assert_eq!(direct.graph["residual_frame"]["shifts_tested"],202);
        let initial_rotation=direct.rotation_residue.clone();
        for _ in 0..2{direct.advance().unwrap();}
        let expected_rotation=vox_core::morphism_factor::modulo(
            &vox_core::morphism_factor::mul(&initial_rotation,&vox_core::morphism_factor::decimal_to_tape("4").unwrap()),
            &direct.rotation_modulus);
        assert_eq!(direct.rotation_residue,expected_rotation,"relative rotation must advance one modular coordinate per carrier step");
        let held=direct.held_record().unwrap();
        let mut resumed=GodelComplexCarrier::from_held_record(&held).unwrap();
        for _ in 0..2{direct.advance().unwrap();resumed.advance().unwrap();}
        let a=direct.held_record().unwrap();let b=resumed.held_record().unwrap();
        assert_eq!(a,b,"segmented execution changed the complete numerical carrier");
        assert!(!direct.exchanges.is_empty(),"cycle did not act on arithmetic arms");
        assert!(direct.certificate_pool.values().any(|c|c["gram"].as_array().unwrap().iter()
            .any(|entry|entry[1].as_str().unwrap()!="0")),"no imaginary coherence was measured");
        assert_eq!(a["catalyst"]["resting"],a["catalyst"]["returned"]);
        let mut corrupt=held.clone();
        let key=corrupt["SIC_pool"].as_object().unwrap().keys().next().unwrap().clone();
        corrupt["SIC_pool"][&key]["mass_numerators"][0]=json!("999");
        assert!(GodelComplexCarrier::from_held_record(&corrupt).is_err());
        let mut corrupt=held;
        corrupt["p"][0]=json!(0);
        assert!(GodelComplexCarrier::from_held_record(&corrupt).is_err());
        eprintln!("integrated {}",direct.progress());
    }
}
