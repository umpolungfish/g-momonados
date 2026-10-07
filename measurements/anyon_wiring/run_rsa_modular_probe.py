#!/usr/bin/env python3
"""Run only >=200-bit RSA-style fixtures through emitted arithmetic gates."""
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
PROJECT = HERE.parents[1]
ROOT = PROJECT.parent
DEPS = PROJECT / 'target/release/deps'
SOURCE = HERE / 'rsa_modular_probe.rs'
BINARY = HERE / 'rsa_modular_probe'
args = ['rustc', '+stable', '--edition=2021', '-O', '-C', 'panic=abort', str(SOURCE),
        '-L', 'dependency=' + str(DEPS), '-o', str(BINARY)]
libs = {}
for crate in ['g_momonados', 'num_bigint', 'num_traits']:
    lib = max(DEPS.glob('lib' + crate + '-*.rlib'), key=lambda p: p.stat().st_mtime)
    args.extend(['--extern', crate + '=' + str(lib)])
    libs[crate] = hashlib.sha256(lib.read_bytes()).hexdigest()
subprocess.run(args, check=True)
manifest = {'source_sha256': hashlib.sha256(SOURCE.read_bytes()).hexdigest(),
            'binary_sha256': hashlib.sha256(BINARY.read_bytes()).hexdigest(),
            'libraries_sha256': libs, 'checked_cases': []}
fixtures = json.loads((ROOT / 'Vox/measurements/anyon_wiring/rsa_cases.json').read_text())
with (HERE / 'rsa_modular_probe.log').open('w') as log:
    for case in fixtures:
        assert int(case['n']).bit_length() >= 200
        subprocess.run([str(BINARY), case['n']], stdout=log, stderr=log, check=True, timeout=180)
        manifest['checked_cases'].append({'bits': case['bits'], 'n': case['n']})
        (HERE / 'rsa_modular_manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
