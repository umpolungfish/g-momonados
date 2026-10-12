"""Bake N-only sources, retain executables, and verify output after production."""
import hashlib
import argparse
import os
import json
from pathlib import Path
import secrets
import shutil
import subprocess
import time
from verify_boundary import verify_boundary

import sympy

root = Path(__file__).resolve().parent
workspace = root.parents[2]
project = workspace / 'G-mOMonadOS'
binding = workspace / 'measurements/factor_synthesis_20261011/n.txt'
original = binding.read_bytes()
inputs = root / 'inputs'
inputs.mkdir(exist_ok=True)
verification = root / 'verification'
verification.mkdir(exist_ok=True)
parser = argparse.ArgumentParser()
parser.add_argument('--tag', default='')
parser.add_argument('--gpu', action='store_true')
parser.add_argument('--gpu-limbs', action='store_true')
parser.add_argument('--carrier', action='store_true')
parser.add_argument('--source-carrier', action='store_true')
parser.add_argument('--cases', nargs='+', default=[
    'regression_208', 'fresh_208', 'regression_224', 'fresh_256', 'regression_895'])
args = parser.parse_args()
sources = {}
sources['control_91'] = b'91\n'
(inputs / 'control_91.txt').write_bytes(sources['control_91'])
for name in ['208', '224', '895']:
    source = original if name == '208' else (binding.parent / f'n_{name}.txt').read_bytes()
    (inputs / f'regression_{name}.txt').write_bytes(source)
    sources[f'regression_{name}'] = source
for bits in [208, 256]:
    name = f'fresh_{bits}'
    path = inputs / f'{name}.txt'
    if not path.exists():
        width = bits // 2
        while True:
            p = int(sympy.nextprime(secrets.randbits(width - 1) | (1 << (width - 1))))
            q = int(sympy.nextprime(secrets.randbits(width - 1) | (1 << (width - 1))))
            n = p * q
            if p != q and n.bit_length() == bits and p.bit_length() == q.bit_length() == width:
                if abs(p - q).bit_length() >= width - 3:
                    break
        path.write_text(f'{n}\n')
        (verification / f'{name}.json').write_text(json.dumps(dict(
            p=str(p), q=str(q), n=str(n), bits=bits,
            p_bits=p.bit_length(), q_bits=q.bit_length(),
            gap_bits=abs(p-q).bit_length(), generator='secrets + sympy.nextprime'), indent=2) + '\n')
    sources[name] = path.read_bytes()

try:
    for case in args.cases:
        source = sources[case]
        name = f'{args.tag}_{case}' if args.tag else case
        binding.write_bytes(source)
        environment = dict(os.environ)
        if args.source_carrier: environment['RESIDENT_SOURCE_PORT'] = 'source-carrier'
        elif args.carrier: environment['RESIDENT_SOURCE_PORT'] = 'nested-carrier'
        elif args.gpu_limbs: environment['RESIDENT_SOURCE_PORT'] = 'gpu-limb-ecm'
        elif args.gpu: environment['RESIDENT_SOURCE_PORT'] = 'gpu-word-ecm'
        else: environment.pop('RESIDENT_SOURCE_PORT', None)
        command = ['cargo', 'build', '--release', '--example', 'resident_glut', '--offline']
        if args.carrier or args.source_carrier: command.extend(['--features', 'carrier-observation'])
        build = subprocess.run(command,
                               cwd=project, env=environment, capture_output=True, text=True)
        (root / f'{name}.build.log').write_text(build.stdout + build.stderr)
        if build.returncode:
            raise RuntimeError(f'{name} build failed: {build.stderr}')
        binary = root / name
        shutil.copy2(project / 'target/release/examples/resident_glut', binary)
        snapshot = root / f'{name}.source'
        snapshot.mkdir(exist_ok=True)
        for source_path in ['Vox/src/sieve.rs', 'Vox/src/morphism_factor.rs',
                            'G-mOMonadOS/src/resident_glut.rs',
                            'G-mOMonadOS/examples/resident_glut.rs']:
            target = snapshot / source_path
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(workspace / source_path, target)
        n = int(source)
        print(f'START {name} bits={n.bit_length()} bound=90s', flush=True)
        started = time.monotonic()
        try:
            run = subprocess.run([str(binary)], cwd=project, capture_output=True, text=True, timeout=90)
            status, code, stdout, stderr = 'returned', run.returncode, run.stdout, run.stderr
        except subprocess.TimeoutExpired as error:
            status, code = 'timeout', None
            stdout = (error.stdout or b'').decode()
            stderr = (error.stderr or b'').decode()
        elapsed = time.monotonic() - started
        (root / f'{name}.stdout').write_text(stdout)
        (root / f'{name}.stderr').write_text(stderr)
        divisors, quotients, verified = verify_boundary(n, stdout)
        ports = [line for line in stderr.splitlines() if line.startswith('source-port')]
        preparation = [line for line in stderr.splitlines() if line.startswith('prepared')]
        clocks = [line for line in stdout.splitlines() if line.startswith('trigger_to_')]
        producer = [line for line in stdout.splitlines() if line.startswith('producer ')]
        receipt = dict(n=str(n), bits=n.bit_length(), runtime_arguments=[],
            source_sha256=hashlib.sha256(source).hexdigest(),
            source_binding_verified=f'source {n}' in stderr.splitlines(),
            binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
            budget_seconds=90, driver_seconds=elapsed, status=status, returncode=code,
            preparation=preparation, production_clocks=clocks, producer=producer,
            product_verified=verified, divisors=[str(d) for d in divisors],
            quotient=str(quotients[0]) if len(quotients) == 1 else None,
            emitted_boundary_verified=verified,
            source_returns=len(ports), last_ports=ports[-4:],
            compiled_source_port='source-carrier' if args.source_carrier else ('nested-carrier' if args.carrier else (
                'gpu-limb-ecm' if args.gpu_limbs else (
                    'gpu-word-ecm' if args.gpu else 'mpqs-dialectic'))))
        (root / f'{name}.receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
        print(json.dumps(receipt), flush=True)
finally:
    binding.write_bytes(original)
