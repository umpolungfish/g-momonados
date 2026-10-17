//! Cargo-side compilation of the source-bound device image and Lean Y tree.
use std::{format, string::String, vec, vec::Vec};
use crate::word_tape::WordTape;

pub fn compile(source: &WordTape, project: &std::path::Path) -> Result<String,String> {
    let plan=vox_core::dialectic_lean::ResidentY::certified_program();
    let line=|key:&str|->Result<&str,String>{plan.lines().find_map(|s|s.strip_prefix(key))
        .ok_or_else(||format!("Lean Y export omitted {key}"))};
    let token=|s:&str|->Result<i32,String>{match s {
        "I"=>Ok(1),"K"=>Ok(2),"S"=>Ok(3),"OP"=>Ok(4),"PAY"=>Ok(5),"@"=>Ok(8),
        _=>Err(format!("unsupported Lean Y token {s}")),
    }};
    let mut tags=vec![6];let mut left=vec![0];let mut right=vec![0];let mut stack=Vec::new();
    let initial=format!("{} OP @ PAY @",line("Y-postfix=")?);
    for t in initial.split_whitespace() {
        let tag=token(t)?;let (l,r)=if tag==8 {
            let r=stack.pop().ok_or("Y tree missing argument")?;
            let l=stack.pop().ok_or("Y tree missing function")?;(l,r)
        }else{(0,0)};
        stack.push(tags.len() as i32);tags.push(tag);left.push(l);right.push(r);
    }
    if stack.len()!=1{return Err("Lean Y tree did not compile to one root".into());}
    let prefix:usize=line("Y-prefix-contractions=")?.parse().map_err(|_|"invalid Y prefix count")?;
    let mut offsets=vec![0];let mut tokens=Vec::new();
    for step in 0..=prefix {
        for t in line(&format!("Y-prefix-{step}="))?.split_whitespace(){tokens.push(token(t)?);}
        offsets.push(tokens.len() as i32);
    }
    let array=|values:&[i32]|values.iter().map(i32::to_string).collect::<Vec<_>>().join(",");
    let width=source.bit_len()+2;
    let marks:Vec<char>=source.as_word().chars().collect();
    let arms:Vec<i32>=(0..width).map(|i|if i<source.bit_len()&&marks[1+5*i+3]=='⊥'{9}else{8}).collect();
    let header=format!("#define W {width}\n#define Y_NODE_COUNT {}\n#define Y_ROOT {}\n#define Y_RESUME 0\n#define Y_PREFIX {prefix}\n__constant__ int Y_TAG[]={{{}}};\n__constant__ int Y_LEFT[]={{{}}};\n__constant__ int Y_RIGHT[]={{{}}};\n__constant__ int Y_OFFSETS[]={{{}}};\n__constant__ int Y_TOKENS[]={{{}}};\n__constant__ unsigned char SOURCE_ARMS[]={{{}}};\n",
        tags.len(),stack[0],array(&tags),array(&left),array(&right),array(&offsets),array(&tokens),array(&arms));
    // Reuse the arithmetic implementation, without its unrelated ECM kernels.
    let arithmetic=include_str!("gpu_ecm_word.cu");
    let before_support=arithmetic.split("// One warp evaluates").next().ok_or("missing GPU arithmetic")?;
    let gcd_start=arithmetic.find("// n is odd, so binary gcd").ok_or("missing GPU gcd")?;
    let gcd_end=arithmetic[gcd_start..].find("__device__ __noinline__ bool inverse").ok_or("missing GPU gcd end")?+gcd_start;
    let cuda=format!("{header}\n{before_support}\n{}\n__device__ bool proper(const Word &g,const Word &n){{return !zero(g)&&!one(g)&&compare(g,n)<0;}}\n{}",
        &arithmetic[gcd_start..gcd_end],include_str!("idempotent_gpu.cu"));
    std::fs::write(project.join("membrane.cu"),&cuda).map_err(|e|e.to_string())?;
    let options=cudarc::nvrtc::CompileOptions{options:vec!["--std=c++17".into()],..Default::default()};
    let ptx=cudarc::nvrtc::compile_ptx_with_opts(cuda,options).map_err(|e|format!("idempotent GPU compilation: {e}"))?;
    Ok(ptx.to_src())
}
