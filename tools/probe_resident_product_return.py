#!/usr/bin/env python3
"""Bake and measure native inverse-product return on an existing prepared source."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
from verify_dialect_factor import numeral

ROOT=Path(__file__).resolve().parents[2]


def main(parent,output):
    output=output.resolve()
    if (output/'receipt.json').exists():raise ValueError('completed source return probe already exists')
    (output/'src').mkdir(parents=True,exist_ok=True)
    modules={'vox':ROOT/'Vox/src/vox.rs','fixed_point_imasm':ROOT/'Vox/src/fixed_point_imasm.rs',
             'fixed_point_word_arithmetic':ROOT/'Vox/src/fixed_point_word_arithmetic.rs',
             'product_return':Path(__file__).with_name('resident_product_return.rs')}
    code='#![allow(dead_code)]\nextern crate alloc;\n'
    for name,path in modules.items():code+=f'#[path={json.dumps(str(path))}] mod {name};\n'
    code+='const SOURCE:&str=include_str!('+json.dumps(str(parent/'source.imasm'))+');\n'
    code+='const OPS:&[u8]=include_bytes!('+json.dumps(str(parent/'relation.ops'))+');\n'
    code+='const REGISTER:&[u8]=include_bytes!('+json.dumps(str(parent/'register.cells'))+');\n'
    code+='''fn main(){
let started=std::time::Instant::now();
let width=SOURCE.chars().filter(|c|matches!(c,'⊤'|'⊥')).count();let mut p=vec![3;width];let mut q=p.clone();
for (wire,raw) in OPS.chunks_exact(16).enumerate(){let op=u32::from_le_bytes(raw[..4].try_into().unwrap());
 if op==8{let variable=u32::from_le_bytes(raw[4..8].try_into().unwrap()) as usize;
 if variable&1==0{p[variable/2]=REGISTER[wire];}else{q[variable/2]=REGISTER[wire];}}}
let source=fixed_point_imasm::ImasmNumeralWord::checked(SOURCE.trim()).unwrap();
let returned=product_return::refine(&source,&p,&q);
println!("{}",serde_json::json!({"bank":returned.bank,"p_before":p,"q_before":q,"p_after":returned.p,"q_after":returned.q,"seconds":started.elapsed().as_secs_f64()}));
}\n'''
    (output/'src/main.rs').write_text(code)
    (output/'Cargo.toml').write_text('[package]\nname="resident-product-return-probe"\nversion="0.1.0"\nedition="2021"\n[dependencies]\nserde_json="1"\n')
    env=dict(os.environ,CARGO_TARGET_DIR=str(output.parent/'product_return_cargo'),RUSTFLAGS='-C target-feature=+crt-static')
    with (output/'build.log').open('wb') as log:
        subprocess.run(['cargo','build','--offline','--release','--target','x86_64-unknown-linux-gnu','--manifest-path',str(output/'Cargo.toml')],env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
    binary=output.parent/'product_return_cargo/x86_64-unknown-linux-gnu/release/resident-product-return-probe'
    with (output/'return.json').open('wb') as log:subprocess.run([str(binary)],stdout=log,check=True)
    result=json.loads((output/'return.json').read_text());bank=result['bank'];n=numeral(bank['source_word'])
    for item in bank['quotient_returns']:
        d,f,r=map(numeral,(item['denominator'],item['floor'],item['remainder']))
        if not d*f+r==n or not 0<=r<d:raise ValueError('native source inverse-product return failed independent division check')
    changes=[]
    for lane in ('p','q'):
        for bit,(before,after) in enumerate(zip(result[lane+'_before'],result[lane+'_after'])):
            if after&~before or not after:raise ValueError('invalid native interval support return')
            if before!=after:changes.append({'lane':lane,'bit':bit,'before':before,'after':after})
    bindings={str(path):hashlib.sha256(path.read_bytes()).hexdigest() for path in [*modules.values(),parent/'source.imasm',parent/'relation.ops',parent/'register.cells',Path(__file__)]}
    receipt={'source_bits':n.bit_length(),'seconds':result['seconds'],'factor_bit_deductions':changes,
             'native_quotient_product_returns_verified':True,'bindings':bindings,
             'scope':'source-only inverse-product interval return on the actual prepared lanes'}
    (output/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('parent',type=Path);p.add_argument('output',type=Path)
    a=p.parse_args();main(a.parent.resolve(),a.output)
