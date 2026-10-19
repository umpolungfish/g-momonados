#!/usr/bin/env python3
"""Reproduce the resident correlation, rollback and nested-return instrument."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT=Path(__file__).resolve().parents[2]


def main(program,output):
    output=output.resolve();(output/'src').mkdir(parents=True,exist_ok=False)
    modules={'vox':'Vox/src/vox.rs','fixed_point_imasm':'Vox/src/fixed_point_imasm.rs',
             'fixed_point_word_arithmetic':'Vox/src/fixed_point_word_arithmetic.rs',
             'factor_relation':'G-mOMonadOS/src/factor_relation.rs',
             'bridge_engine':'G-mOMonadOS/tools/relational_imasm_bridge.rs',
             'joint_cells':'G-mOMonadOS/tools/relational_joint_cells.rs'}
    reentry=ROOT/'G-mOMonadOS/src/reentry_powerset.rs'
    # Omit only this dependency's unrelated amplitude-arena test module.
    (output/'reentry.rs').write_text(reentry.read_text().split('#[cfg(test)]')[0])
    source='#![allow(dead_code)]\nextern crate alloc;\n'
    for name,path in modules.items():source+=f'#[path={json.dumps(str(ROOT/path))}] mod {name};\n'
    source+=f'#[path={json.dumps(str(output/"reentry.rs"))}] mod reentry_powerset;\nfn main(){{}}\n'
    (output/'src/main.rs').write_text(source)
    (output/'Cargo.toml').write_text('''[package]
name="resident-dialect-instrument"
version="0.1.0"
edition="2021"
[dependencies]
serde_json="1"
serde="1"
num-bigint="0.4"
num-traits="0.2"
flate2="1"
''')
    env=dict(os.environ,NATIVE_ARITHMETIC_PROGRAM=str(program.resolve()),
             RESIDENT_EVIDENCE_TEST_DIR=str(output/'evidence_test'),CARGO_TARGET_DIR=str(output.parent/'native_instrument_cargo'))
    with (output/'unit.log').open('wb') as log:
        result=subprocess.run(['cargo','test','--offline','--release','--manifest-path',str(output/'Cargo.toml'),
                               'resident_','--','--nocapture'],env=env,stdout=log,stderr=subprocess.STDOUT)
    if result.returncode:raise subprocess.CalledProcessError(result.returncode,'native resident instrument')
    paths=[ROOT/path for path in modules.values()]+[reentry,Path(__file__),
          Path(__file__).with_name('resident_dialect_relations.rs'),
          Path(__file__).with_name('resident_product_return.rs'),
          Path(__file__).with_name('resident_reader_index.rs'),Path(__file__).with_name('resident_evidence_store.rs'),program]
    receipt={'passed':True,'instrument':'native row-pair correlation, interval fibres, prepared reader incidence, static recovery, rollback and nested fuse',
             'bindings':{str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths},
             'log_sha256':hashlib.sha256((output/'unit.log').read_bytes()).hexdigest()}
    (output/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('program',type=Path);p.add_argument('output',type=Path)
    a=p.parse_args();main(a.program,a.output)
