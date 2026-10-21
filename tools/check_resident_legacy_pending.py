#!/usr/bin/env python3
"""Reproduce old pending work on a fresh N-only source, then compile its continuation."""
import argparse
import hashlib
import json
import secrets
import subprocess
import sys
import shutil
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
TOOLS=Path(__file__).resolve().parent
SNAPSHOT=Path('/mnt/d/imsgct-progress/artifacts/resident_archived_product_20261019/locked512/source_snapshot')
TABLES=Path('/mnt/d/imsgct-progress/artifacts/dialect_small_controls_fixed_20261018/n187.source/dialect_tables.json')
NATIVE=Path('/mnt/d/imsgct-progress/artifacts/imasm_type_bridge_current_20261018/execution.json')


def digest(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def main(output,snapshot,reuse_control=None):
    from sympy import isprime
    from verify_dialect_factor import numeral_word
    from check_prepared_factor_witness import check
    from compare_resident_recovery import compare
    from run_resident_dialect_integration import audit_output
    output=output.resolve();output.mkdir(exist_ok=False)
    pinned=output/'legacy_tools';pinned.mkdir()
    binding=json.loads((snapshot.parent/'launch_binding.json').read_text())
    expected={Path(name).name:sha for name,sha in binding['sources'].items()}
    common={'vox.rs':'Vox/src/vox.rs','fixed_point_imasm.rs':'Vox/src/fixed_point_imasm.rs',
            'fixed_point_word_arithmetic.rs':'Vox/src/fixed_point_word_arithmetic.rs',
            'factor_relation.rs':'G-mOMonadOS/src/factor_relation.rs',
            'reentry_powerset.rs':'G-mOMonadOS/src/reentry_powerset.rs'}
    restored=[]
    for original in sorted(snapshot.iterdir()):
        name=original.name.split('_',1)[1]
        if digest(original)!=expected[name]:raise ValueError('snapshot differs from its executable source binding: '+name)
        row={'source':str(original),'source_sha256':digest(original)}
        if name in common:
            current=ROOT/common[name]
            if digest(current)!=expected[name]:raise ValueError('shared source differs from the frozen legacy compiler: '+name)
            row.update(shared_source=str(current))
        else:
            text=original.read_text()
            if original.suffix=='.py':
                text=text.replace('ROOT = Path(__file__).resolve().parents[2]',
                                  'ROOT = Path('+json.dumps(str(ROOT))+')')
                text=text.replace('return hashlib.sha256(path.read_bytes()).hexdigest()',
                                  'with path.open("rb") as stream:\n        return hashlib.file_digest(stream,"sha256").hexdigest()')
                text=text.replace('return hashlib.sha256(Path(path).read_bytes()).hexdigest()',
                                  'with Path(path).open("rb") as stream:\n        return hashlib.file_digest(stream,"sha256").hexdigest()')
            actual=pinned/name;actual.write_text(text)
            row.update(pinned_source=str(actual),pinned_sha256=digest(actual),
                       adaptation='project root paths and streamed SHA256 only' if digest(actual)!=expected[name] else 'byte-identical')
        restored.append(row)
    (output/'legacy_source_binding.json').write_text(json.dumps(restored,indent=2)+'\n')
    inputs=output/'inputs';inputs.mkdir();references=output/'verification_only';references.mkdir()
    def prime():
        while True:
            value=secrets.randbits(104)|(1<<103)|1
            if isprime(value):return value
    source=inputs/'fresh_balanced_208.imasm'
    reference=references/'fresh_balanced_208.json'
    if reuse_control is None:
        while True:
            p,q=prime(),prime();n=p*q
            if p!=q and n.bit_length()==208:break
        source.write_text(numeral_word(n)+'\n')
        reference.write_text(json.dumps({'p':str(p),'q':str(q),'source_sha256':digest(source),
            'scope':'verification only; excluded from both compilers and executables'},indent=2)+'\n')
    else:
        shutil.copyfile(reuse_control/'inputs/fresh_balanced_208.imasm',source)
        shutil.copyfile(reuse_control/'verification_only/fresh_balanced_208.json',reference)
    legacy=output/'legacy';resumed=output/'resumed'
    flags=['--tables',str(TABLES),'--native-bridges',str(NATIVE),'--joint-cells',
           '--resident-dialects','--source-product-return','--nested-reentry']
    with (output/'legacy_bake.stdout').open('wb') as stdout,(output/'legacy_bake.stderr').open('wb') as stderr:
        subprocess.run([sys.executable,str(pinned/'bake_relational_imasm_bridge.py'),str(source),str(legacy),
                        *flags,'--seconds','30'],stdout=stdout,stderr=stderr,check=True)
    old=json.loads((legacy/'return.json').read_text())
    if old['status']!='held' or old.get('control_word') is not None:
        raise ValueError('legacy instrument did not retain the expected pending execution')
    (legacy/'prepared_reference_check.json').write_text(json.dumps(check(legacy,reference),indent=2)+'\n')
    subprocess.run([sys.executable,str(TOOLS/'verify_relational_imasm_bridge.py'),str(legacy),
                    str(legacy/'verification.json')],check=True)
    print(json.dumps({'stage':'legacy source-only capture certified','phase':old['bridge_phase'],
                      'nested':old['continuation'].get('nested_frame') is not None}),flush=True)
    subprocess.run([sys.executable,str(TOOLS/'bake_relational_imasm_bridge.py'),str(source),str(resumed),
                    *flags,'--resume',str(legacy/'return.json'),'--seconds','2'],check=True)
    compare(legacy/'return.json',resumed)
    (resumed/'prepared_reference_check.json').write_text(json.dumps(check(resumed,reference),indent=2)+'\n')
    audit_output(resumed)
    result={'passed':True,'source_sha256':digest(source),'legacy_return_sha256':digest(legacy/'return.json'),
            'resumed_return_sha256':digest(resumed/'return.json'),
            'legacy_source_binding_sha256':digest(output/'legacy_source_binding.json'),
            'recovery':json.loads((resumed/'recovery_comparison.json').read_text()),
            'scope':'fresh balanced208 legacy capture, exact continuation recovery and current-word execution',
            'instrument_sha256':digest(Path(__file__))}
    (output/'receipt.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output',type=Path)
    parser.add_argument('--snapshot',type=Path,default=SNAPSHOT)
    parser.add_argument('--reuse-control',type=Path,help='reuse the N-only input after a pre-execution harness failure')
    args=parser.parse_args();main(args.output,args.snapshot,args.reuse_control)
