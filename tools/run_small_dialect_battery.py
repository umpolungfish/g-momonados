#!/usr/bin/env python3
"""Exercise explicit small N-only controls through the current baked membrane."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
from verify_dialect_factor import numeral

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('destination', type=Path)
    parser.add_argument('--kernel', type=Path, required=True)
    parser.add_argument('--tables', type=Path, required=True)
    args = parser.parse_args()
    results = []
    flags = ['--keep-fibres', '--compose-idempotent', '--hop', '--product-frame',
             '--cofactor-feedback', '--support-newton', '--promotion-return',
             '--support-secant', '--support-parity', '--support-frame-point',
             '--environment-carry', '--mixed-environment-operations']
    for n in [15, 21, 35, 91, 187, 221]:
        binary = args.destination / (f'n{n}_r2' if n == 15 else f'n{n}')
        source = args.destination / 'inputs' / f'{n}.imasm'
        prefix = Path(str(binary) + '_run1')
        project = Path(str(binary) + '.source')
        commands = [
            [sys.executable, 'tools/bake_dialect_factor.py', source, binary,
             '--kernel', args.kernel, '--small-control-tables', args.tables, *flags],
            [sys.executable, 'tools/run_dialect_factor.py', binary, prefix, '--seconds', '1'],
            [sys.executable, 'tools/verify_dialect_factor.py', project,
             Path(str(prefix) + '.json'), Path(str(prefix) + '.verification.json')],
        ]
        for index, command in enumerate(commands):
            with Path(str(binary) + f'.stage{index}.log').open('wb') as log:
                subprocess.run(list(map(str, command)), cwd=ROOT, stdout=log,
                               stderr=subprocess.STDOUT, check=True)
        verified = json.loads(Path(str(prefix) + '.verification.json').read_text())
        pairs = sorted({(numeral(p['divisor_word']), numeral(p['quotient_word']))
                        for p in verified['closed_pairs']})
        assert pairs, f'small control {n} returned no divisor'
        for divisor, quotient in pairs:
            assert 1 < divisor < n and n % divisor == 0 and divisor * quotient == n
        prepared = json.loads((project / 'prepared.json').read_text())
        result = {'N': n, 'pairs': pairs, 'zero_remainder': True, 'exact_product': True,
                  'producing_boundary': 'source-parity preparation' if prepared['receipt'][3] == 8 else 'dialect transport and terminal source fuse',
                  'transitions_checked': verified['transitions_checked'],
                  'return_sha256': verified['receipt_sha256'],
                  'verifier_sha256': verified['verifier_sha256']}
        results.append(result)
        (args.destination / 'battery.json').write_text(json.dumps({
            'complete': len(results) == 6, 'results': results,
            'harness_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}, indent=2) + '\n')
        print(json.dumps(result), flush=True)


if __name__ == '__main__':
    main()
