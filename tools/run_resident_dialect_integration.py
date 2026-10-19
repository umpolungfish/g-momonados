#!/usr/bin/env python3
"""Run source-only resident dialect preparation, execution and independent audit."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import secrets
import subprocess
import sys
import time

ROOT=Path(__file__).resolve().parents[2]
TOOLS=Path(__file__).resolve().parent
TABLES=Path('/mnt/d/imsgct-progress/artifacts/dialect_small_controls_fixed_20261018/n187.source/dialect_tables.json')
NATIVE=Path('/mnt/d/imsgct-progress/artifacts/imasm_type_bridge_current_20261018/execution.json')


def run(stage,base):
    from verify_dialect_factor import numeral_word
    if stage=='control':
        from sympy import isprime
        inputs=base/'inputs';references=base/'verification_only';inputs.mkdir(parents=True,exist_ok=True);references.mkdir(exist_ok=True)
        source=inputs/'fresh_balanced_208.imasm'
        if source.exists():raise ValueError('fresh control already exists; use its continuation')
        def prime():
            while True:
                value=secrets.randbits(104)|(1<<103)|1
                if isprime(value):return value
        while True:
            p,q=prime(),prime();n=p*q
            if p!=q and n.bit_length()==208:break
        source.write_text(numeral_word(n)+'\n')
        (references/'fresh_balanced_208.json').write_text(json.dumps({'p':str(p),'q':str(q),'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),
            'scope':'verification only; never passed to the compiler or membrane'},indent=2)+'\n')
        output=base/'fresh208';seconds=30;resume=None
    elif stage=='resume':
        source=base/'inputs/fresh_balanced_208.imasm';output=base/'fresh208_resume';seconds=2
        resume=base/'fresh208/return.json'
    else:
        source=ROOT/'G-mOMonadOS/measurements/resident_imasm_integration_20261012/inputs/fresh_balanced_512.imasm'
        output=base/'locked512';seconds=300;resume=None
    command=[sys.executable,str(TOOLS/'bake_relational_imasm_bridge.py'),str(source),str(output),
             '--tables',str(TABLES),'--native-bridges',str(NATIVE),'--joint-cells','--resident-dialects',
             '--seconds',str(seconds)]
    if resume:command+=['--resume',str(resume)]
    subprocess.run(command,check=True)
    subprocess.run([sys.executable,str(TOOLS/'verify_relational_imasm_bridge.py'),str(output),str(output/'verification.json')],check=True)
    # Altered native rule banks and changed packed continuation bytes must be
    # rejected by the same independent audit that accepts the original.
    from verify_relational_imasm_bridge import verify
    negative=[]
    original=json.loads((output/'return.json').read_text())
    for label in ('native_type_bank','trail_previous_mask','source_boundary'):
        changed=copy.deepcopy(original)
        if label=='native_type_bank':changed['resident_dialect']['native_types']['4'][0][0]^=1
        elif label=='trail_previous_mask':
            import gzip,struct
            c=changed['continuation'];raw=bytearray(gzip.decompress(bytes.fromhex(c['trail_gzip_hex'])))
            if not raw:raise ValueError('execution did not exercise continuation')
            # A previous empty mask cannot contain the executing contraction.
            raw[8]=0;c['trail_gzip_hex']=gzip.compress(raw).hex()
        else:
            # A source alteration tests the actual immutable source boundary.
            changed['source_word']+='⊙'
        try:verify(output,changed)
        except (ValueError,AssertionError,IndexError,KeyError):negative.append({'control':label,'rejected':True})
        else:raise ValueError('altered resident evidence was accepted: '+label)
    (output/'negative_controls.json').write_text(json.dumps(negative,indent=2)+'\n')
    print(json.dumps({'stage':stage,'directory':str(output),'negative_controls':negative}))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('stage',choices=['control','resume','locked'])
    p.add_argument('--output',type=Path,default=Path('/mnt/d/imsgct-progress/artifacts/resident_dialect_integration_20261018'))
    args=p.parse_args();run(args.stage,args.output)
