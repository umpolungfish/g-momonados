#!/usr/bin/env python3
"""Verify physical braid contraction and the resident operator handoff."""
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
PROJECT = HERE.parents[1]
ROOT = PROJECT.parent
DEPS = PROJECT / 'target/release/deps'
SOURCE = HERE / 'rsa_operator_probe.rs'
BINARY = HERE / 'rsa_operator_probe'
args = ['rustc', '+stable', '--edition=2021', '-O', '-C', 'panic=abort',
        str(SOURCE), '-L', 'dependency=' + str(DEPS), '-o', str(BINARY)]
libraries = {}
build = subprocess.run(['cargo', '+stable', 'build', '--offline', '--release',
                        '--lib', '--bin', 'ququart_factor', '--message-format=json'],
                       cwd=PROJECT, check=True, capture_output=True, text=True)
artifacts = {}
for line in build.stdout.splitlines():
    message = json.loads(line)
    if message.get('reason') == 'compiler-artifact':
        for filename in message['filenames']:
            if filename.endswith('.rlib'):
                artifacts[message['target']['name']] = Path(filename)
for crate in ['g_momonados', 'num_bigint', 'num_traits', 'serde_json']:
    # Use the dependency graph of this build, rather than a newer unrelated
    # test artifact with a different Rust crate identity or panic profile.
    library = artifacts[crate]
    libraries[crate] = hashlib.sha256(library.read_bytes()).hexdigest()
    args.extend(['--extern', crate + '=' + str(library)])
subprocess.run(args, check=True)
manifest = {'probe_sha256': hashlib.sha256(SOURCE.read_bytes()).hexdigest(),
            'binary_sha256': hashlib.sha256(BINARY.read_bytes()).hexdigest(),
            'support_sha256': hashlib.sha256((PROJECT / 'src/bin/ququart_support/mod.rs').read_bytes()).hexdigest(),
            'libraries_sha256': libraries, 'cases': []}
with (HERE / 'rsa_operator_entry_tests.log').open('w') as log:
    for case in json.loads((ROOT / 'Vox/measurements/anyon_wiring/rsa_cases.json').read_text()):
        assert int(case['n']).bit_length() >= 200
        braid = HERE / f"rsa_fourier_{case['bits']}.word"
        prepared = HERE / f"rsa_physical_operator_{case['bits']}.json"
        subprocess.run([str(BINARY), case['n'], str(braid), str(prepared)],
                       stdout=log, stderr=log, timeout=240, check=True)
        log.flush()
        manifest['cases'].append({'bits': case['bits'], 'n': case['n'],
                                 'word_sha256': hashlib.sha256(braid.read_bytes()).hexdigest(),
                                 'operator_sha256': hashlib.sha256(prepared.read_bytes()).hexdigest()})
        (HERE / 'rsa_operator_entry_manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
