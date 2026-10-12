"""Bake N-only sources, retain executables, and verify output after production."""
import hashlib
import argparse
import gzip
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
word_binding = binding.with_suffix('.imasm')

def numeral_word(source):
    value = int(source)
    return ('⊢' + ''.join('≻⋈∈' + ('⊥' if value >> bit & 1 else '⊤') + '∋'
                         for bit in range(value.bit_length())) + '⊙⊡⊣\n').encode()

original_word = word_binding.read_bytes() if word_binding.exists() else numeral_word(original)
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
parser.add_argument('--shared-constraints', action='store_true')
parser.add_argument('--joint-phase', action='store_true')
parser.add_argument('--bound', type=float, default=90)
parser.add_argument('--cases', nargs='+', default=[
    'regression_208', 'fresh_208', 'regression_224', 'fresh_256', 'regression_895'])
parser.add_argument('--input-root', type=Path, default=inputs)
parser.add_argument('--output-root', type=Path, default=root)
args = parser.parse_args()
root = args.output_root.resolve()
root.mkdir(parents=True, exist_ok=True)
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
    if name not in args.cases:
        continue
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

for case in args.cases:
    if case not in sources:
        if Path(case).name != case:
            raise ValueError('case names must refer to N-only files in inputs')
        path = args.input_root / f'{case}.txt'
        source = path.read_bytes()
        if not source.strip().isdigit() or int(source) < 2:
            raise ValueError(f'{case} is not a source numeral greater than one')
        sources[case] = source

try:
    for case in args.cases:
        source = sources[case]
        name = f'{args.tag}_{case}' if args.tag else case
        binding.write_bytes(source)
        baked_word = numeral_word(source)
        word_binding.write_bytes(baked_word)
        environment = dict(os.environ)
        if args.joint_phase: environment['RESIDENT_SHARED_CONSTRAINTS'] = 'joint-phase'
        elif args.shared_constraints: environment['RESIDENT_SHARED_CONSTRAINTS'] = 'product-prefix'
        else: environment.pop('RESIDENT_SHARED_CONSTRAINTS', None)
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
        (snapshot / 'source.txt').write_bytes(source)
        (snapshot / 'source.imasm').write_bytes(baked_word)
        for source_path in ['Vox/src/sieve.rs', 'Vox/src/morphism_factor.rs',
                            'G-mOMonadOS/src/resident_glut.rs',
                            'Vox/src/structural_factor_oracle.rs',
                            'G-mOMonadOS/src/whole_register_phase.rs',
                            'G-mOMonadOS/src/ququart_decision.rs',
                            'G-mOMonadOS/src/word_tape.rs',
                            'G-mOMonadOS/src/anyon_ququart.rs',
                            'G-mOMonadOS/src/phase_unbraid.rs',
                            'G-mOMonadOS/src/reversible_modular.rs',
                            'Vox/src/fixed_point_word_arithmetic.rs',
                            'G-mOMonadOS/src/gpu_ecm_word.rs',
                            'G-mOMonadOS/src/gpu_ecm_word.cu',
                            'G-mOMonadOS/examples/resident_glut.rs']:
            target = snapshot / source_path
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(workspace / source_path, target)
        n = int(source)
        print(f'START {name} bits={n.bit_length()} bound={args.bound}s', flush=True)
        started = time.monotonic()
        try:
            run = subprocess.run([str(binary)], cwd=project, capture_output=True, text=True, timeout=args.bound)
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
            source_word_sha256=hashlib.sha256(baked_word).hexdigest(),
            source_word_binding_verified=f'source-word {baked_word.decode().strip()}' in stderr.splitlines(),
            binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
            budget_seconds=args.bound, driver_seconds=elapsed, status=status, returncode=code,
            preparation=preparation, production_clocks=clocks, producer=producer,
            preparation_stages=[line for line in stderr.splitlines() if line.startswith('preparation-stage')],
            sic_evidence=[line for line in stderr.splitlines() if line.startswith((
                'SIC initial residual=', 'SIC rational residual_numerator_bits=',
                'SIC rational denominator=', 'SIC coherence-control'))],
            shared_constraints=[line for line in stderr.splitlines() if line.startswith('shared-product-prefix')],
            joint_phase=[line for line in stderr.splitlines() if line.startswith('joint-resident-phase')],
            product_verified=verified, divisors=[str(d) for d in divisors],
            quotient=str(quotients[0]) if len(quotients) == 1 else None,
            emitted_boundary_verified=verified,
            source_returns=len(ports), last_ports=ports[-4:],
            compiled_source_port='resident-product-phase' if args.joint_phase else ('source-carrier' if args.source_carrier else ('nested-carrier' if args.carrier else (
                'gpu-limb-ecm' if args.gpu_limbs else (
                    'gpu-word-ecm' if args.gpu else 'mpqs-dialectic')))))
        if args.source_carrier:
            vox = workspace / 'Vox/target/release/vox'
            wirer = project / 'tools/vox_binary_rewire.py'
            anchor = root / 'dispatch_20261012_resident_glut_anchor.diagrammar.txt'
            if not vox.is_file():
                raise RuntimeError(f'Vox executable is required to bind {name}: {vox}')
            disassembly = root / f'{name}.vox.disasm'
            wiring = root / f'{name}.wiring.tsv'
            with disassembly.open('w') as output:
                subprocess.run([str(vox), 'disasm', str(binary)], check=True,
                               stdout=output, stderr=subprocess.PIPE, text=True)
            with wiring.open('w') as output:
                subprocess.run([str(vox), 'disasm', str(binary), '--wiring-complete', '--wiring'],
                               check=True, stdout=output, stderr=subprocess.PIPE, text=True)
            rewired = root / f'{name}.rewired.json.gz'
            rewire_log = root / f'{name}.rewire.log'
            rewire = subprocess.run([
                'python3', str(wirer), '--elf', str(binary), '--vox', str(vox),
                '--diagrammar', str(anchor), '--output', str(rewired),
                '--diagrammar-output', str(root / f'{name}.rewired.diagrammar.txt.gz'),
                '--imasm-output', str(root / f'{name}.rewired.imasm')],
                cwd=project, capture_output=True, text=True)
            rewire_log.write_text(rewire.stdout + rewire.stderr)
            if rewire.returncode:
                raise RuntimeError(f'{name} Vox binary wiring failed: {rewire.stdout}{rewire.stderr}')
            carrier_line = next((line for line in stderr.splitlines()
                                 if line.startswith('carrier word=')), None)
            if carrier_line is None:
                raise RuntimeError(f'{name} did not report its executed source-carrier word')
            carrier_word = carrier_line.split('carrier word=', 1)[1].split(' vox-control-flow-verdict=', 1)[0]
            carrier = subprocess.run([str(vox), 'construct-carrier', carrier_word],
                                     check=True, capture_output=True, text=True)
            with gzip.open(rewired, 'rt') as report:
                rewiring_checks = json.load(report)['checks']
            binding_report = dict(binary_sha256=receipt['binary_sha256'],
                source_sha256=receipt['source_sha256'], carrier_word=carrier_word,
                carrier_tower=carrier.stdout.strip(), disassembly=str(disassembly),
                wiring=str(wiring), rewire_log=str(rewire_log),
                rewiring_checks=rewiring_checks,
                rewired_binary_word=str(root / f'{name}.rewired.imasm'))
            binding_path = root / f'{name}.vox-binding.json'
            binding_path.write_text(json.dumps(binding_report, indent=2) + '\n')
            receipt['vox_binding'] = binding_report
        (root / f'{name}.receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
        print(json.dumps(receipt), flush=True)
finally:
    binding.write_bytes(original)
    word_binding.write_bytes(original_word)
