#!/usr/bin/env python3
"""Check actual emitted braids across the source-derived modular register height."""
import hashlib
import json
from pathlib import Path
import subprocess
HERE = Path(__file__).resolve().parent
PROJECT = HERE.parents[1]
ROOT = PROJECT.parent
DEPS = PROJECT/'target/release/deps'
SOURCE = HERE/'rsa_frame_probe.rs'
BINARY = HERE/'rsa_frame_probe'
args=['rustc','+stable','--edition=2021','-O','-C','panic=abort',str(SOURCE),
      '-L','dependency='+str(DEPS),'-o',str(BINARY)]
libraries={}
for crate in ['g_momonados','num_bigint']:
    lib=max(DEPS.glob('lib'+crate+'-*.rlib'),key=lambda p:p.stat().st_mtime)
    libraries[crate]=hashlib.sha256(lib.read_bytes()).hexdigest()
    args.extend(['--extern',crate+'='+str(lib)])
subprocess.run(args,check=True)
manifest={'probe_sha256':hashlib.sha256(SOURCE.read_bytes()).hexdigest(),
          'libraries_sha256':libraries,'cases':[]}
with (HERE/'rsa_physical_frame_audit.log').open('w') as log:
    for case in json.loads((ROOT/'Vox/measurements/anyon_wiring/rsa_cases.json').read_text()):
        assert int(case['n']).bit_length()>=200
        word=HERE/f"rsa_fourier_{case['bits']}.word"
        subprocess.run([str(BINARY),case['n'],str(word)],stdout=log,stderr=log,timeout=180,check=True)
        log.flush()
        manifest['cases'].append({'bits':case['bits'],'n':case['n'],
            'word_sha256':hashlib.sha256(word.read_bytes()).hexdigest()})
        (HERE/'rsa_frame_manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
