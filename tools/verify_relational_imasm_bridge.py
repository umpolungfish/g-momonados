#!/usr/bin/env python3
"""Audit a returned relational carrier, and independently check emitted factors."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
from verify_dialect_factor import numeral


def verify(directory, altered_report=None):
    report = altered_report if altered_report is not None else json.loads((directory / 'return.json').read_text())
    binding = json.loads((directory / 'binding.json').read_text())
    source = (directory / 'source.imasm').read_text().strip()
    if report['source_word'] != source:
        raise ValueError('source bank changed')
    for filename, key in [('source.imasm', 'source_sha256'), ('membrane', 'executable_sha256'),
                          ('return.json', 'return_sha256'), ('relation.ops', 'relation_sha256'),
                          ('register.cells', 'prepared_register_sha256'),
                          ('relation.cells', 'joint_cells_sha256'), ('cell.rows', 'native_cell_rows_sha256')]:
        if key in binding and hashlib.sha256((directory / filename).read_bytes()).hexdigest() != binding[key]:
            raise ValueError(f'changed bound artifact: {filename}')
    packed = (directory / 'relation.ops').read_bytes()
    ops = list(struct.iter_unpack('<IIII', packed))
    prepared = list((directory / 'register.cells').read_bytes())
    domains = report['register']
    if not (len(ops) == len(prepared) == len(domains)) or any(d not in [1, 2, 3] for d in domains):
        raise ValueError('invalid executing register dimensions or alternatives')
    inputs = {}
    joint_cells = []
    if (directory / 'relation.cells').exists():
        joint_cells = list(struct.iter_unpack('<IIIII', (directory / 'relation.cells').read_bytes()))
        if joint_cells:
            from analyze_product_cell_fusion import cells
            expected_ports = [ports for _, ports in cells(ops)]
            if joint_cells != expected_ports:
                raise ValueError('joint cell lost its actual lowered arithmetic ports')
        rows = (directory / 'cell.rows').read_bytes()
        expected_rows = []
        for index in range(8):
            bits = [(index >> i) & 1 for i in range(3)]
            expected_rows.extend([1 if b else 2 for b in bits] +
                                 [1 if sum(bits) % 2 else 2, 1 if sum(bits) >= 2 else 2])
        if list(rows) != expected_rows:
            raise ValueError('native joint sum/carry relation changed')
        if report.get('joint_cells', 0) != len(joint_cells):
            raise ValueError('executing joint cell bank differs')
    for wire, (op, a, b, required) in enumerate(ops):
        if op not in [0, 3, 4, 5, 7, 8]:
            raise ValueError('unknown relation operator')
        if op in [3, 4, 7] and not (a < wire and (op == 3 or b < wire)):
            raise ValueError('relation lost its retained dependency graph')
        if op == 8:
            inputs[a] = wire
        if (op == 0 and domains[wire] != 2) or ((op == 5 or required) and domains[wire] != 1):
            raise ValueError('source boundary or constant changed')
    if sorted(inputs) != list(range(len(inputs))):
        raise ValueError('unknown factor lanes incomplete')
    restored = domains[:]
    for wire, previous in reversed(report['continuation']['trail']):
        if not (0 <= wire < len(ops)) or previous not in [1, 2, 3] or restored[wire] & ~previous:
            raise ValueError('noncontracting or malformed continuation trail')
        restored[wire] = previous
    if restored != prepared:
        raise ValueError('continuation does not return to the actual prepared register')
    at = 0
    for wire, alternative, boundary in report['continuation']['decisions']:
        if wire not in inputs.values() or alternative not in [0, 2] or not at <= boundary <= len(report['continuation']['trail']):
            raise ValueError('branch continuation changed')
        if domains[wire] != (1 if alternative else 2):
            raise ValueError('selected branch lost its correlated assignment')
        at = boundary
    for gate in report['continuation']['pending_gates']:
        if not 0 <= gate < len(ops) + len(joint_cells) or (
                gate < len(ops) and ops[gate][0] not in [3, 4, 7]):
            raise ValueError('invalid pending relation operation')
    if binding.get('parent_return_sha256'):
        raw = (directory / 'parent_return.json').read_bytes()
        if hashlib.sha256(raw).hexdigest() != binding['parent_return_sha256']:
            raise ValueError('retained parent return changed')
        parent = json.loads(raw)
        if parent['status'] != 'held' or parent['source_word'] != source:
            raise ValueError('continuation parent is not the same open source')
        for counter in ['branches', 'contractions', 'rows_examined', 'bridge_applications']:
            if report[counter] < parent[counter]:
                raise ValueError('retained execution counters were restarted')
    provenance = json.loads((directory / 'bridge_provenance.json').read_text())
    if 'tables_path' in provenance:
        from bake_relational_imasm_bridge import bridges
        _, independently_rebuilt = bridges(Path(provenance['tables_path']),
                                           Path(provenance['native_execution_receipt_path']))
        if provenance != independently_rebuilt:
            raise ValueError('native rule action or bridge provenance changed')
    image = [1, 2]
    phase = report['bridge_phase']
    hops = [hop['native_hop_image'] for hop in provenance['ancestry']]
    seen = {}
    tick = 0
    while tick < phase:
        key = (tick % len(hops), tuple(image))
        if key in seen:
            period = tick - seen[key]
            skip = (phase - tick) // period
            if skip:
                tick += skip * period
                continue
        seen[key] = tick
        image = [hops[tick % len(hops)][value] for value in image]
        tick += 1
    if image != report['carried_T_F_images']:
        raise ValueError('bridge effects were not carried across hops')
    closure = report['closure']
    values_checked = 0
    if closure is not None:
        n = numeral(source)
        p = numeral(closure['divisor_word'])
        q = numeral(closure['quotient_word'])
        if not (1 < p < n and n % p == 0 and n // p == q and p*q == n):
            raise ValueError('emitted divisor or exact source product is invalid')
        values = []
        for wire, (op, a, b, required) in enumerate(ops):
            value = {0: False, 5: True}.get(op)
            if op == 8:
                value = bool(((p if a % 2 == 0 else q) >> (a//2)) & 1)
            elif op == 3:
                value = not values[a]
            elif op == 4:
                value = values[a] and values[b]
            elif op == 7:
                value = values[a] or values[b]
            if not domains[wire] & (1 if value else 2) or (required and not value):
                raise ValueError('emitted pair violates a retained source relation')
            values.append(value)
        values_checked = len(values)
    return {'source_bits': numeral(source).bit_length(), 'status': report['status'],
            'proper_factor': closure is not None, 'emitted_assignment_gate_checks': values_checked,
            'retained_wires': len(ops), 'prepared_register_recovered_from_trail': True,
            'joint_cells_checked': len(joint_cells),
            'branch_continuation_checked': True, 'carried_native_bridge_image_checked': True,
            'scope': 'assignment and continuation audit',
            'return_sha256': hashlib.sha256((directory / 'return.json').read_bytes()).hexdigest()}


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('directory', type=Path)
    p.add_argument('output', type=Path)
    args = p.parse_args()
    result = verify(args.directory)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result))
