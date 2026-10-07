#!/usr/bin/env python3
"""Observe resident execution with the actual contracted Fourier braid."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

HERE = Path(__file__).resolve().parent
PROJECT = HERE.parents[1]
ROOT = PROJECT.parent
BINARY = PROJECT / 'target/release/ququart_factor'
options = argparse.ArgumentParser()
options.add_argument('--radix', type=int, default=2)
options = options.parse_args()
assert options.radix >= 2 and options.radix & (options.radix - 1) == 0
radix_word = json.loads(subprocess.run(
    [str(PROJECT / 'target/release/ququart_prepare_operator'), '--numerals', str(options.radix)],
    check=True, capture_output=True, text=True).stdout)[str(options.radix)]
stem = f'rsa_physical_ququart_radix{options.radix}'
records = []
for case in json.loads((ROOT / 'Vox/measurements/anyon_wiring/rsa_cases.json').read_text()):
    assert int(case['n']).bit_length() >= 200
    operator = HERE / f"rsa_physical_operator_{case['bits']}.json"
    prepared = json.loads(operator.read_text())
    prepared['radix_word'] = radix_word
    runtime_input = HERE / f"{stem}_{case['bits']}_input.json"
    runtime_input.write_text(json.dumps(prepared) + '\n')
    env = os.environ.copy()
    env['QUQUART_TRACE'] = '1'
    started = time.monotonic()
    try:
        result = subprocess.run([str(BINARY), case['n'], str(runtime_input)], env=env,
                                capture_output=True, text=True, timeout=30)
        stdout, stderr, code = result.stdout, result.stderr, result.returncode
    except subprocess.TimeoutExpired as observation:
        stdout = (observation.stdout or b'').decode()
        stderr = (observation.stderr or b'').decode()
        code = None
    pair = re.search(r'= (\d+) x (\d+)', stdout)
    verified = bool(pair and int(pair[1]) > 1 and int(pair[2]) > 1
                    and int(pair[1]) * int(pair[2]) == int(case['n'])
                    and {int(pair[1]), int(pair[2])} == {int(case['p']), int(case['q'])})
    records.append({'bits': case['bits'], 'n': case['n'], 'returncode': code,
                    'fourier_mode': 'contracted_physical',
                    'work_radix': options.radix,
                    'resident_started': 'ququart_counters=' in stderr,
                    'product_verified': verified, 'stdout': stdout, 'stderr': stderr,
                    'seconds': time.monotonic() - started,
                    'binary_sha256': hashlib.sha256(BINARY.read_bytes()).hexdigest(),
                    'operator_sha256': hashlib.sha256(operator.read_bytes()).hexdigest(),
                    'runtime_input_sha256': hashlib.sha256(runtime_input.read_bytes()).hexdigest()})
    (HERE / f"{stem}_{case['bits']}.trace").write_text(stderr)
    (HERE / f'{stem}.json').write_text(json.dumps(records, indent=2) + '\n')
if not all(record['product_verified'] for record in records):
    raise SystemExit(1)
