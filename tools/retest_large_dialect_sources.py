#!/usr/bin/env python3
"""Retest the large N-only sources only after the corrected small battery passes."""
import argparse
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('small_battery', type=Path)
    parser.add_argument('destination', type=Path)
    parser.add_argument('--kernel', type=Path, required=True)
    parser.add_argument('--control-source', type=Path, required=True)
    parser.add_argument('--locked-source', type=Path, required=True)
    parser.add_argument('--parent-return', type=Path, required=True)
    parser.add_argument('--parent-verification', type=Path, required=True)
    args = parser.parse_args()
    small = json.loads(args.small_battery.read_text())
    assert small['complete'] and {r['N'] for r in small['results']} == {15, 21, 35, 91, 187, 221}
    for r in small['results']:
        assert r['pairs'] and r['zero_remainder'] and r['exact_product']
        for p, q in r['pairs']:
            assert 1 < p < r['N'] and p * q == r['N'] and r['N'] % p == 0
    args.destination.mkdir(parents=True, exist_ok=True)
    flags = ['--keep-fibres', '--compose-idempotent', '--hop', '--product-frame',
             '--cofactor-feedback', '--support-newton', '--promotion-return',
             '--support-secant', '--support-parity', '--support-frame-point',
             '--environment-carry', '--mixed-environment-operations']
    completed = []
    for name, source, seconds, resume in [
            ('control_208_updated', args.control_source, 30, False),
            ('locked_512_r25', args.locked_source, 300, True)]:
        binary = args.destination / name
        project = Path(str(binary) + '.source')
        prefix = args.destination / (name + '_run' + str(seconds))
        build = [sys.executable, 'tools/bake_dialect_factor.py', source, binary,
                 '--kernel', args.kernel, *flags]
        parent = []
        if resume:
            build += ['--resume-return', args.parent_return,
                      '--resume-verification', args.parent_verification]
            parent = ['--parent-verification', args.parent_verification]
        stages = [
            ('compile', build),
            ('prepare_verify', [sys.executable, 'tools/verify_dialect_factor.py', project,
              project / 'instrument.json', Path(str(binary) + '.instrument_verification.json'),
              '--negative-controls', *parent]),
            ('payload_verify', [sys.executable, 'tools/verify_dialect_payload.py', binary,
              project, Path(str(binary) + '.payload_verification.json')]),
            ('run', [sys.executable, 'tools/run_dialect_factor.py', binary, prefix,
                     '--seconds', str(seconds)]),
            ('return_verify', [sys.executable, 'tools/verify_dialect_factor.py', project,
              Path(str(prefix) + '.json'), Path(str(prefix) + '.verification.json'),
              '--negative-controls', *parent]),
        ]
        for stage, command in stages:
            print(json.dumps({'case': name, 'stage': stage, 'status': 'started'}), flush=True)
            with Path(str(binary) + '.' + stage + '.log').open('wb') as log:
                subprocess.run(list(map(str, command)), cwd=ROOT, stdout=log,
                               stderr=subprocess.STDOUT, check=True)
            print(json.dumps({'case': name, 'stage': stage, 'status': 'complete'}), flush=True)
        result = json.loads(Path(str(prefix) + '.verification.json').read_text())
        completed.append({'case': name, 'return_sha256': result['receipt_sha256'],
                          'closed_pairs': result['closed_pairs'],
                          'transitions_checked': result['transitions_checked']})
        (args.destination / 'large_retests.json').write_text(json.dumps({
            'complete': len(completed) == 2, 'small_battery': str(args.small_battery),
            'results': completed}, indent=2) + '\n')


if __name__ == '__main__':
    main()
