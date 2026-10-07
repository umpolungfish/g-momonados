#!/usr/bin/env python3
"""Observe resident ququart execution only on >=200-bit RSA-style sources."""
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
records = []
for case in json.loads((ROOT / 'Vox/measurements/anyon_wiring/rsa_cases.json').read_text()):
    assert int(case['n']).bit_length() >= 200
    env = os.environ.copy()
    env['QUQUART_TRACE'] = '1'
    started = time.monotonic()
    try:
        result = subprocess.run([str(BINARY), case['n']], env=env, capture_output=True,
                                text=True, timeout=30)
        stdout, stderr, code = result.stdout, result.stderr, result.returncode
    except subprocess.TimeoutExpired as error:
        stdout, stderr = (error.stdout or b'').decode(), (error.stderr or b'').decode()
        code = None
    pair = re.search(r'= (\d+) x (\d+)', stdout)
    verified = bool(pair and {int(pair[1]), int(pair[2])} == {int(case['p']), int(case['q'])}
                    and int(pair[1]) * int(pair[2]) == int(case['n']))
    record = {'bits': case['bits'], 'n': case['n'], 'returncode': code,
              'stdout': stdout, 'stderr': stderr, 'product_verified': verified,
              'seconds': time.monotonic() - started,
              'binary_sha256': hashlib.sha256(BINARY.read_bytes()).hexdigest()}
    records.append(record)
    (HERE / 'rsa_ququart_interleaved.json').write_text(json.dumps(records, indent=2) + '\n')
    (HERE / f"rsa_ququart_{case['bits']}.trace").write_text(stderr)
