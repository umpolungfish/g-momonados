#!/usr/bin/env python3
"""Audit current anyon stack binaries and bind their wiring to exact ELF hashes."""
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
PROJECT = HERE.parents[1]
ROOT = PROJECT.parent
OUT = HERE / 'stack_recovery'
AUDITOR = PROJECT / 'target/release/anyon_stack_audit'


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def wiring(binary, name):
    destination = OUT / name
    with (OUT / (name + '_wiring.log')).open('w') as log:
        subprocess.run(['python3', str(ROOT / 'render_membrane_diagram.py'),
                        str(binary), '--out', str(destination)],
                       stdout=log, stderr=log, check=True)
    graph = json.loads((destination / 'circuit.json').read_text())
    assert graph['sha256'] == digest(binary)
    boundaries = []
    for function in graph['functions']:
        names = ' '.join(function['names'])
        if any(term in names for term in ('fuse_source_arms', 'fuse_interval_arms',
                                         'verify_recovery', 'require_clean_workspace',
                                         'controlled_multiply', 'audit_frobenius_stack',
                                         'source_cofactors', 'partition_less')):
            boundaries.append({'address': function['id'], 'names': function['names']})
    return {'binary': str(binary), 'sha256': graph['sha256'],
            'atlas': str(destination / 'index.html'), 'boundaries': boundaries}


def main():
    OUT.mkdir(exist_ok=True)
    manifest = {'question': 'Do transformed arms recover at every anyon stack fuse and phase stage?',
                'auditor_sha256': digest(AUDITOR), 'cases': [], 'binaries': []}
    for case in json.loads((ROOT / 'Vox/measurements/anyon_wiring/rsa_cases.json').read_text()):
        source = int(case['n'])
        assert source.bit_length() >= 200 and int(case['p']) * int(case['q']) == source
        prepared = HERE / f"rsa_physical_ququart_radix16_{case['bits']}_input.json"
        braid = HERE / f"rsa_fourier_{case['bits']}.word"
        report_path = OUT / f"rsa_{case['bits']}.json"
        command = [str(AUDITOR), str(prepared), '2', str(braid)]
        print('auditing', case['bits'], 'bits', flush=True)
        with report_path.open('w') as report, (OUT / f"rsa_{case['bits']}.log").open('w') as log:
            subprocess.run(command, stdout=report, stderr=log, check=True)
        report = json.loads(report_path.read_text())
        assert report['source'] == case['n']
        assert [stage['height'] for stage in report['stages']] == list(range(case['bits'] + 4))
        assert report['presentation_heights'] == 3 * case['bits'] + 5
        assert report['fusion_channels'] == 5
        for stage in report['stages']:
            assert all(stage[field] for field in ('transformed_state_recovered',
                                                  'adjacent_inverse_return', 'workspace_clean'))
        manifest['cases'].append({'bits': case['bits'], 'source': case['n'],
                                  'prepared_sha256': digest(prepared), 'braid_sha256': digest(braid),
                                  'report': str(report_path), 'report_sha256': digest(report_path),
                                  'command': command})
        (OUT / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
        print('recovered every stage for', case['bits'], 'bits', flush=True)
    for name in ('anyon_stack_audit', 'anyon_generator', 'ququart_factor'):
        print('diagramming', name, flush=True)
        manifest['binaries'].append(wiring(PROJECT / 'target/release' / name, name))
        (OUT / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    stock = [(PROJECT / 'target/release' / name, name)
             for name in ('anyon_cnot_compile', 'ququart_prepare_operator',
                          'ququart_verify_readout', 'g-momonados')]
    stock += [(OUT / f'{name}_{bits}', f'{name}_{bits}_wiring')
              for bits in (200, 256)
              for name in ('ququart_factor_baked', 'ququart_baked')]
    manifest['stock_binaries'] = [wiring(binary, name) for binary, name in stock]
    (OUT / 'stock_binaries.json').write_text(
        json.dumps(manifest['stock_binaries'], indent=2) + '\n')
    manifest['kernel_sources'] = {
        name: digest(PROJECT / name) for name in
        ('src/ququart_decision.rs', 'src/ququart_folded_work.rs',
         'src/reversible_modular.rs', 'src/bin/anyon_stack_audit.rs')}
    manifest['physical_entry_probe'] = str(HERE / 'rsa_operator_entry_manifest.json')
    (OUT / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print('saved', OUT / 'manifest.json', flush=True)


if __name__ == '__main__':
    main()
