#!/usr/bin/env python3
"""Reproduce the resident correlation, rollback and nested-return instrument."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import copy

ROOT=Path(__file__).resolve().parents[2]


def main(program,output,verified_receipt):
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
    paths=[ROOT/path for path in modules.values()]+[reentry,Path(__file__),
          Path(__file__).with_name('resident_dialect_relations.rs'),
          Path(__file__).with_name('resident_product_return.rs'),
          Path(__file__).with_name('resident_reader_index.rs'),Path(__file__).with_name('resident_evidence_store.rs'),
          Path(__file__).with_name('verify_relational_imasm_bridge.py'),program,verified_receipt]
    bindings={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
    env=dict(os.environ,NATIVE_ARITHMETIC_PROGRAM=str(program.resolve()),
             VERIFIED_FACTOR_RECEIPT=str(verified_receipt.resolve()),
             RESIDENT_EVIDENCE_TEST_DIR=str(output/'evidence_test'),CARGO_TARGET_DIR=str(output.parent/'native_instrument_cargo'))
    with (output/'unit.log').open('wb') as log:
        result=subprocess.run(['cargo','test','--offline','--release','--manifest-path',str(output/'Cargo.toml'),
                               'resident_','--','--nocapture'],env=env,stdout=log,stderr=subprocess.STDOUT)
    if result.returncode:raise subprocess.CalledProcessError(result.returncode,'native resident instrument')
    if bindings!={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}:
        raise ValueError('instrument source or input changed during compilation/execution')
    from verify_relational_imasm_bridge import audit_verification_tail
    from verify_dialect_factor import numeral,numeral_word
    fixture_path=output/'evidence_test/verification_tail.json'
    fixture=json.loads(fixture_path.read_text());inputs=dict(fixture['input_wires'])
    def audit_tail(report):
        def bank_value(bank,slot):
            if bank!=0:raise ValueError('verification-only fixture has no such returned bank')
            return report['register'][slot]
        return audit_verification_tail(report,report['source_word'],inputs,report['register'],bank_value,report['register'])
    checked=[audit_tail(report) for report in fixture['snapshots']]
    negatives=[]
    for label in ('skipped_operation','changed_quotient','changed_product','premature_fixation','changed_return_bank','changed_factor_clock'):
        changed=copy.deepcopy(fixture['snapshots'][0 if label=='premature_fixation' else -1])
        tail=changed['verification_word_control']
        if label=='skipped_operation':tail['executed_positions'].pop(0)
        elif label=='changed_quotient':tail['quotient_word']=numeral_word(numeral(tail['quotient_word'])+1)
        elif label=='changed_product':tail['product_word']=numeral_word(numeral(tail['product_word'])+1)
        elif label=='premature_fixation':changed['holding_frame']['fixed']=True
        elif label=='changed_return_bank':tail['returned_bank']=1
        else:tail['trigger_to_verified_factor_seconds']=-1
        try:audit_tail(changed)
        except (ValueError,AssertionError):negatives.append({'control':label,'rejected':True})
        else:raise ValueError('altered verification-word evidence was accepted: '+label)
    receipt={'passed':True,'instrument':'native row-pair correlation, interval fibres, prepared reader incidence, static recovery, rollback and nested fuse',
             'bindings':bindings,'verification_tail_audit':checked,'verification_tail_negative_controls':negatives,
             'verification_fixture_sha256':hashlib.sha256(fixture_path.read_bytes()).hexdigest(),
             'log_sha256':hashlib.sha256((output/'unit.log').read_bytes()).hexdigest()}
    (output/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n');print(json.dumps(receipt))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('program',type=Path);p.add_argument('output',type=Path)
    p.add_argument('--verified-factor-receipt',type=Path,default=ROOT/'measurements/factor_synthesis_20261011/receipt_y_lazy_208.json',
                   help='previously emitted factor receipt, used only by verification-tail tests; never a producer input')
    a=p.parse_args();main(a.program,a.output,a.verified_factor_receipt)
