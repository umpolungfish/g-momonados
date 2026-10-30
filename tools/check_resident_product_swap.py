#!/usr/bin/env python3
"""Check native inverse-product return under swapping actual retained lanes."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT=Path(__file__).resolve().parents[2]


def main(parent,output):
    report=json.loads((parent/'return.json').read_text())
    banks=report['source_product_return']['banks']
    index=next(i for i in range(len(banks)-1,-1,-1) if banks[i]['p_before']!=banks[i]['q_before'])
    carried=banks[index];output=output.resolve();(output/'src').mkdir(parents=True,exist_ok=False)
    paths={'vox':ROOT/'Vox/src/vox.rs','fixed_point_imasm':ROOT/'Vox/src/fixed_point_imasm.rs',
           'fixed_point_word_arithmetic':ROOT/'Vox/src/fixed_point_word_arithmetic.rs',
           'product_return':Path(__file__).with_name('resident_product_return.rs')}
    source='#![allow(dead_code)]\nextern crate alloc;\n'
    for name,path in paths.items():source+=f'#[path={json.dumps(str(path))}] mod {name};\n'
    source+='const SOURCE:&str=include_str!('+json.dumps(str(parent/'source.imasm'))+');\n'
    for name,key in [('P','p_before'),('Q','q_before')]:source+='const '+name+':&[u8]=&'+json.dumps(carried[key])+';\n'
    source+='''fn main(){let source=fixed_point_imasm::ImasmNumeralWord::checked(SOURCE.trim()).unwrap();
let a=product_return::refine(&source,P,Q);let b=product_return::refine(&source,Q,P);
assert_eq!(a.p,b.q);assert_eq!(a.q,b.p);
for name in ["min","max","lower","upper"]{assert_eq!(a.bank[format!("p_{name}")],b.bank[format!("q_{name}")]);assert_eq!(a.bank[format!("q_{name}")],b.bank[format!("p_{name}")]);}
for (i,j) in [(0,2),(1,3),(2,0),(3,1)]{assert_eq!(a.bank["quotient_returns"][i],b.bank["quotient_returns"][j]);}
println!("{}",serde_json::json!({"swap_equivariant":true,"distinct_input_lanes":P!=Q,"returned_p":a.p,"returned_q":a.q,"bank":a.bank}));}\n'''
    (output/'src/main.rs').write_text(source)
    (output/'Cargo.toml').write_text('[package]\nname="native-product-swap-instrument"\nversion="0.1.0"\nedition="2021"\n[dependencies]\nserde_json="1"\n')
    env=dict(os.environ,CARGO_TARGET_DIR=str(output.parent/'product_swap_cargo'),RUSTFLAGS='-C target-feature=+crt-static')
    with (output/'build.log').open('wb') as log:
        subprocess.run(['cargo','build','--offline','--release','--target','x86_64-unknown-linux-gnu','--manifest-path',str(output/'Cargo.toml')],env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
    binary=output.parent/'product_swap_cargo/x86_64-unknown-linux-gnu/release/native-product-swap-instrument'
    with (output/'return.json').open('wb') as log:subprocess.run([str(binary)],stdout=log,check=True)
    actual=json.loads((output/'return.json').read_text())
    if actual['returned_p']!=carried['p_after'] or actual['returned_q']!=carried['q_after']:
        raise ValueError('native re-execution differs from saved product return')
    receipt={'passed':True,'parent_return_sha256':hashlib.sha256((parent/'return.json').read_bytes()).hexdigest(),
             'bank_index':index,'distinct_carried_lanes':True,'source_product_swap_equivariant':True,
             'bindings':{str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in [*paths.values(),Path(__file__)]}}
    (output/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('parent',type=Path);p.add_argument('output',type=Path)
    a=p.parse_args();main(a.parent.resolve(),a.output)
