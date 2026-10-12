"""Execute retained GPU source-bound binaries and preserve separate device receipts."""
import hashlib
import argparse
import json
import os
from pathlib import Path
import subprocess
import time
from verify_boundary import verify_boundary

root = Path(__file__).resolve().parent
project = root.parents[2] / 'G-mOMonadOS'
parser = argparse.ArgumentParser()
parser.add_argument('--prefix', default='gpu')
args = parser.parse_args()
for case in ['control_91', 'fresh_208', 'regression_224', 'fresh_256', 'regression_895']:
    binary = root / f'{args.prefix}_{case}'
    binding = json.loads((root / f'{args.prefix}_{case}.receipt.json').read_text())
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == binding['binary_sha256']
    n = int(binding['n'])
    started = time.monotonic()
    print(f'START device_{case} bound=90s', flush=True)
    try:
        run = subprocess.run([str(binary)], cwd=project,
            env=dict(os.environ, ECM_TIMING='1'), capture_output=True, text=True, timeout=90)
        stdout, stderr, code = run.stdout, run.stderr, run.returncode
        status = 'returned' if code == 0 else 'error'
    except subprocess.TimeoutExpired as error:
        stdout = (error.stdout or b'').decode()
        stderr = (error.stderr or b'').decode()
        code, status = None, 'timeout'
    elapsed = time.monotonic() - started
    divisors, quotients, verified = verify_boundary(n, stdout)
    if status == 'returned' and not verified: status = 'no-divisor'
    receipt = dict(n=str(n), bits=n.bit_length(), runtime_arguments=[], budget_seconds=90,
        binary_sha256=binding['binary_sha256'], driver_seconds=elapsed, status=status,
        returncode=code, product_verified=verified, divisors=[str(d) for d in divisors],
        quotient=str(quotients[0]) if len(quotients) == 1 else None,
        emitted_boundary_verified=verified,
        source_binding_verified=f'source {n}' in stderr.splitlines(),
        preparation=[line for line in stderr.splitlines() if line.startswith('prepared')],
        gpu_stages=[line for line in stderr.splitlines() if line.startswith('ECM timing')],
        production_clocks=[line for line in stdout.splitlines() if line.startswith('trigger_to_')],
        producer=[line for line in stdout.splitlines() if line.startswith('producer ')])
    name = f'{args.prefix}_device_{case}'
    (root / f'{name}.stdout').write_text(stdout)
    (root / f'{name}.stderr').write_text(stderr)
    (root / f'{name}.receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt), flush=True)
    if case == 'control_91' and not verified:
        raise RuntimeError(f'GPU closure control failed: {stderr[-1000:]}')
