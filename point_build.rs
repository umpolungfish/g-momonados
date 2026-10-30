//! Cargo-only emission of the prepared point circuit and its shared banks.
use std::{fs,io::{self,Write},path::Path};
use serde_json::Value;
use crate::word_tape::SignedWordTape;

fn text(v:&Value)->&str{v.as_str().expect("compiled text")}
fn signed(v:&Value)->String{
    let v=SignedWordTape::from_prepared_record(v).expect("prepared coefficient");
    format!("BakedSignedWord{{negative:{},word:{:?}}}",v.is_negative(),v.magnitude().as_word())
}
fn complex(v:&Value)->String{format!("BakedWordComplex{{re:{},im:{}}}",signed(&v["re"]),signed(&v["im"]))}
fn integer(v:&Value)->String{
    let v=SignedWordTape::from_prepared_record(v).expect("prepared coordinate").to_bigint();
    let (sign,digits)=v.to_u32_digits();
    format!("BakedInteger{{negative:{},digits:&{digits:?}}}",sign==num_bigint::Sign::Minus)
}
fn pair_list(v:&Value)->String{
    v.as_array().unwrap().iter().map(|row|format!("({}, {})",row[0],row[1])).collect::<Vec<_>>().join(",")
}
fn carriers(v:&Value)->String{
    v.as_array().unwrap().iter().map(|row|format!(
        "Reg16_3{{big_t:{},big_f:{},small_t:{},small_f:{}}}",row[0],row[1],row[2],row[3]))
        .collect::<Vec<_>>().join(",")
}

pub fn emit(record:&Value,out_dir:&Path)->io::Result<()>{
    let mut out=io::BufWriter::new(fs::File::create(out_dir.join("point_prepared.rs"))?);
    writeln!(out,"use g_momonados::{{baked_point::*,baked_resident::*}};")?;
    writeln!(out,"use imasm_core::{{classic::Token,imasm16_3::Reg16_3}};")?;
    let arena=&record["register"];
    writeln!(out,"static POINT_NODES:&[BakedNode]=&[")?;
    for node in arena["nodes"].as_array().unwrap(){
        if node.get("re").is_some(){writeln!(out,"BakedNode::Leaf({}),",complex(node))?;}
        else{writeln!(out,"BakedNode::Branch{{wire:{},low:{},high:{}}},",node["wire"],node["low"],node["high"])?;}
    }
    writeln!(out,"];")?;
    let mut bytes=Vec::new();let mut spans=Vec::new();
    for mask in arena["support_wires"].as_array().unwrap(){
        let start=bytes.len()/8;
        for wire in mask.as_array().unwrap(){bytes.extend(wire.as_u64().unwrap().to_le_bytes());}
        spans.push((start,bytes.len()/8));
    }
    fs::write(out_dir.join("point_support.bin"),&bytes)?;
    writeln!(out,"#[cfg(not(target_endian=\"little\"))] compile_error!(\"prepared point support requires little-endian transport\");")?;
    writeln!(out,"static POINT_SUPPORT_WIRES:[u64;{}]=unsafe{{core::mem::transmute(*include_bytes!(concat!(env!(\"OUT_DIR\"),\"/point_support.bin\")))}};",bytes.len()/8)?;
    writeln!(out,"static POINT_SUPPORT:BakedSparseSupport=BakedSparseSupport{{wires:&POINT_SUPPORT_WIRES,spans:&{spans:?},fixed:&[")?;
    for row in arena["fixed"].as_array().unwrap(){writeln!(out,"({},{},{}),",row[0],row[1],row[2])?;}
    writeln!(out,"]}};")?;
    let projector=&record["projector"];
    writeln!(out,"static POINT_PROJECTOR:BakedPointProjector=BakedPointProjector{{scopes:&[")?;
    for scope in projector["scopes"].as_array().unwrap(){writeln!(out,"&{scope},")?;}
    writeln!(out,"],incidence:&[")?;
    for row in projector["incidence"].as_array().unwrap(){writeln!(out,"({},&{}),",row[0],row[1])?;}
    writeln!(out,"],internal:&{},first_internal:{},versions:&[{}],pending:&[",projector["internal"],projector["first_internal"],pair_list(&projector["versions"]))?;
    for row in projector["pending"].as_array().unwrap(){writeln!(out,"({},{},{}),",row[0],row[1],row[2])?;}
    writeln!(out,"],constraints:{},contractions:{},copy_returns:&[",projector["constraints"],projector["contractions"])?;
    for row in projector["copy_returns"].as_array().unwrap(){writeln!(out,"({},{},{}),",row[0],row[1],row[2])?;}
    writeln!(out,"]}};")?;
    let protocol=&record["protocol"];let sic=&protocol["sic"];
    writeln!(out,"static POINT_SIC:BakedSic=BakedSic{{modulus_bits:{},rays:&[",sic["modulus_bits"])?;
    for ray in sic["rays"].as_array().unwrap(){
        write!(out,"[")?;for z in ray.as_array().unwrap(){write!(out,"BakedComplex{{re:{},im:{}}},",integer(&z["re"]),integer(&z["im"]))?;}
        writeln!(out,"],")?;
    }
    writeln!(out,"],projectors:&[")?;
    for row in sic["projectors"].as_array().unwrap(){
        write!(out,"[")?;for z in row.as_array().unwrap(){write!(out,"{},",complex(z))?;}writeln!(out,"],")?;
    }
    writeln!(out,"]}};")?;
    writeln!(out,"static POINT_PROTOCOL:BakedPointProtocol=BakedPointProtocol{{sic:&POINT_SIC,modulus_bits:{},w_bits:{},",protocol["modulus_bits"],protocol["w_bits"])?;
    writeln!(out,"dual_denominator:{},dual_columns:&[",integer(&protocol["dual_denominator"]))?;
    for row in protocol["dual_columns"].as_array().unwrap(){
        write!(out,"[")?;for z in row.as_array().unwrap(){write!(out,"BakedComplex{{re:{},im:{}}},",integer(&z["re"]),integer(&z["im"]))?;}
        writeln!(out,"],")?;
    }
    writeln!(out,"],")?;
    for key in ["central_step","engagement"]{
        write!(out,"{key}:&[")?;for z in protocol[key].as_array().unwrap(){write!(out,"{},",complex(z))?;}writeln!(out,"],")?;
    }
    writeln!(out,"disassembler:{:?},recompiler:{:?},forward:{},backward:{},program_self_return:{},executed_trie_steps:{}}};",
        text(&protocol["disassembler"]),text(&protocol["recompiler"]),protocol["forward"],protocol["backward"],protocol["program_self_return"],protocol["executed_trie_steps"])?;
    writeln!(out,"static POINT_GRAPHS:[BakedPointGraph;2]=[")?;
    for graph in record["graphs"].as_array().unwrap(){
        let nodes=graph["node_variants"].as_array().unwrap().iter().map(|v|format!("Token::{}",text(v))).collect::<Vec<_>>().join(",");
        writeln!(out,"BakedPointGraph{{nodes:&[{nodes}],edges:&[{}],word:{:?},reference_inputs:&[{}],reference_edges:&[{}]}},",
            pair_list(&graph["edges"]),text(&graph["word"]),carriers(&graph["reference_inputs"]),carriers(&graph["reference_edges"]))?;
    }
    writeln!(out,"];")?;
    let y=&record["y"];
    let trace=y["prefix_trace"].as_array().unwrap().iter().map(|v|format!("{:?}",text(v))).collect::<Vec<_>>().join(",");
    writeln!(out,"static POINT_Y:BakedPointY=BakedPointY{{postfix:&{},prefix_trace:&[{trace}],prefix_contractions:{},contractions:{},calls:{},done:{}}};",
        y["postfix"],y["prefix_contractions"],y["contractions"],y["calls"],y["done"])?;
    let runtime=&record["runtime_state"];
    writeln!(out,"static POINT_RUNTIME:BakedPointRuntime=BakedPointRuntime{{row:{},column:{},subtract:{},reduced:{},projected:{},operations:{},protocol_bank_count:{},protocol_returns:{:?}}};",
        runtime["row"],runtime["column"],runtime["subtract"],runtime["reduced"],runtime["projected"],runtime["operations"],
        runtime["protocol_bank_count"],serde_json::to_string(&runtime["protocol_returns"])?)?;
    writeln!(out,"static POINT:BakedPoint=BakedPoint{{source_digits:&{},source_word:{:?},hex_word:{:?},",record["source_digits"],text(&record["source_word"]),text(&record["hex_word"]))?;
    let digits=record["binary_digits"].as_array().unwrap().iter().map(|v|format!("{:?}",text(v).chars().next().unwrap())).collect::<Vec<_>>().join(",");
    writeln!(out,"binary_digits:&[{digits}],powers:&[")?;
    for power in record["powers"].as_array().unwrap(){writeln!(out,"&{power},")?;}
    writeln!(out,"],complement_word:{:?},moment_offset:{},moment_weights:&[",text(&record["complement_word"]),signed(&record["moment_offset"]))?;
    for weight in record["moment_weights"].as_array().unwrap(){writeln!(out,"{},",signed(weight))?;}
    writeln!(out,"],cells:{},roots:&{},nodes:POINT_NODES,support:&POINT_SUPPORT,",arena["cells"],arena["roots"])?;
    writeln!(out,"point_wires:&{},residual_wires:&{},coordinates:{},fork_history:{},engagement_history:{},",
        record["point_wires"],record["residual_wires"],record["coordinates"],record["fork_history"],record["engagement_history"])?;
    writeln!(out,"projector:&POINT_PROJECTOR,protocol:&POINT_PROTOCOL,graphs:&POINT_GRAPHS,y:&POINT_Y,runtime:&POINT_RUNTIME,preparation_nanoseconds:{}}};",record["preparation_nanoseconds"])?;
    writeln!(out,"fn compiled_point_switch()->Result<g_momonados::conventional_spectrum::PreparedPointSwitch,String>{{g_momonados::conventional_spectrum::PreparedPointSwitch::from_baked(&POINT)}}")?;
    out.flush()?;
    fs::write(out_dir.join("point_prepared.expected.json"),serde_json::to_vec(record)?)?;
    let receipt=serde_json::json!({"schema":record["schema"],"input_boundary":"N-only",
        "source_digits":record["source_digits"],"source_word":record["source_word"],
        "register_cells":arena["cells"],"register_nodes":arena["nodes"].as_array().unwrap().len(),
        "retained_roots":arena["roots"].as_array().unwrap().len(),"tensor_banks":projector["scopes"].as_array().unwrap().len(),
        "preparation_nanoseconds":record["preparation_nanoseconds"],
        "Y_calls":y["calls"],"Y_contractions":y["contractions"],"projector_contractions":projector["contractions"],
        "prepared_sic":true,"prepared_native_graphs":true,"prepared_phase_operator":true,"prepared_self_model":true});
    fs::write(out_dir.join("point_preparation_receipt.json"),serde_json::to_vec_pretty(&receipt)?)?;
    eprintln!("Cargo prepared point cells={} nodes={} roots={}",arena["cells"],arena["nodes"].as_array().unwrap().len(),arena["roots"].as_array().unwrap().len());
    Ok(())
}

pub fn source(source:&str)->Result<num_bigint::BigUint,String>{
    let source=source.trim();
    if !source.is_empty()&&source.bytes().all(|c|c.is_ascii_digit()){
        source.parse().map_err(|_|"invalid N-only decimal source".into())
    }else{
        crate::word_tape::WordTape::from_canonical_word(source).map(|v|v.to_biguint())
            .ok_or_else(||"invalid N-only IMASM source".into())
    }
}
