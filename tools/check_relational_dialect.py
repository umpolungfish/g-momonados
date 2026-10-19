#!/usr/bin/env python3
"""Bind native dialect rules to an actual prepared joint arithmetic cell."""
import argparse
from dataclasses import asdict
import json
from pathlib import Path
import struct
from array import array
from relational_dialect import NativeRules, TypedJointRelation, digest


def check(directory, rules):
    source = (directory / 'source.imasm').read_text().strip()
    domains = (directory / 'register.cells').read_bytes()
    native_rows = list((directory / 'cell.rows').read_bytes())
    if len(native_rows) != 40:
        raise ValueError('native adder row bank changed')
    ports = next(struct.iter_unpack('<IIIII', (directory / 'relation.cells').read_bytes()))
    rows = [native_rows[i:i+5] for i in range(0, 40, 5)]
    rows = [row for row in rows if all(domains[port] & v for port, v in zip(ports, row))]
    if not rows:
        raise ValueError('prepared joint cell has no live rows')
    relation = TypedJointRelation.arithmetic(source, ports, rows, rules)
    actions, type_classes, bank_classes, return_classes = [], {}, {}, {}
    for dialect in range(len(rules.tables['tables'])):
        for operation in rules.tables['operation_names']:
            changed = relation.compose_into(rules, dialect, operation, ports[0], ports[1], ports[3])
            assert changed.origins == relation.origins and changed.ports == relation.ports
            assert changed.source_word == source
            for i, row in enumerate(changed.rows):
                for port in (0, 1, 2, 4):
                    assert row[port] == relation.rows[i][port]
            trace = changed.history[-1]['rows']
            for observed in trace:
                assert len(observed['native_execution']) == 12
                for block in observed['native_execution']:
                    assert block['seed'] == relation.rows[observed['row']][3][block['axis']]
            key = (dialect, operation)
            type_classes.setdefault(repr(changed.types), []).append(key)
            bank_classes.setdefault(repr([r['native_execution'] for r in trace]), []).append(key)
            return_classes.setdefault(repr(changed.rows), []).append(key)
            actions.append({'dialect': dialect, 'operation': operation,
                            'type_changed_rows': sum(a != b for a, b in zip(relation.types, changed.types)),
                            'return_changed_rows': sum(a != b for a, b in zip(relation.rows, changed.rows))})
    # Native absorption differences act on the actual same operand objects.
    first = relation.compose_into(rules, 22, 'join', ports[0], ports[1], ports[3])
    canonical = relation.compose_into(rules, 0, 'join', ports[0], ports[1], ports[3])
    assert first.types != canonical.types
    second = first.compose_into(rules, 26, 'meet', ports[3], ports[2], ports[4])
    reset = relation.compose_into(rules, 26, 'meet', ports[3], ports[2], ports[4])
    assert second.history[-1]['rows'][0]['left_type'] == first.types[0][3]
    # The continuation uses the first returned output as its next operand.
    assert [r['left_type'] for r in second.history[-1]['rows']] != [r['left_type'] for r in reset.history[-1]['rows']]
    assert second.origins == relation.origins
    binding_files = ['source.imasm', 'relation.cells', 'cell.rows', 'register.cells']
    report = {'native_tuple_executions_checked': rules.checked_native_compositions,
              'dialects': len(rules.tables['tables']), 'actions': actions,
              'ports': ports, 'live_joint_rows': len(rows),
              'distinct_type_relations': len(type_classes),
              'distinct_native_banks': len(bank_classes),
              'distinct_terminal_relations': len(return_classes),
              'successive_operand_is_actual_return': True,
              'source_and_origins_retained': True,
              'binding': 'explicit input0,input1 -> sum port; then sum,input2 -> carry port',
              'scope': 'bound native dialect composition and type-program execution; arithmetic refinement/extraction not wired',
              'bindings': rules.bindings | {str((directory / p).resolve()): digest(directory / p) for p in binding_files}}
    return report, {'initial': asdict(relation), 'first': asdict(first), 'second': asdict(second)}


def network(directory, rules):
    """Keep the source's compact factor graph, instantiating each joint row frame."""
    binding = json.loads((directory / 'binding.json').read_text())
    names = {'source.imasm': 'source_sha256', 'register.cells': 'prepared_register_sha256',
             'relation.cells': 'joint_cells_sha256', 'cell.rows': 'native_cell_rows_sha256',
             'relation.ops': 'relation_sha256'}
    for filename, key in names.items():
        if digest(directory / filename) != binding[key]:
            raise ValueError(f'changed source network bank: {filename}')
    source = (directory / 'source.imasm').read_text().strip()
    domains = (directory / 'register.cells').read_bytes()
    raw_rows = (directory / 'cell.rows').read_bytes()
    rows = [list(raw_rows[i:i+5]) for i in range(0, len(raw_rows), 5)]
    if len(rows) != 8 or any(len(row) != 5 for row in rows):
        raise ValueError('source network has an invalid joint row bank')
    # Template row IDs remain exact arithmetic assignments; each frame below
    # supplies its actual shared source wires. No complete product is expanded.
    templates, frames, cached = [], [], {}
    incidences = {}
    for ports in struct.iter_unpack('<IIIII', (directory / 'relation.cells').read_bytes()):
        active = tuple(i for i, row in enumerate(rows)
                       if all(domains[port] & v for port, v in zip(ports, row)))
        if active not in cached:
            initial = TypedJointRelation.arithmetic(source, range(5), [rows[i] for i in active], rules)
            first = initial.compose_into(rules, 22, 'join', 0, 1, 3)
            second = first.compose_into(rules, 26, 'meet', 3, 2, 4)
            assert second.origins == initial.origins and second.source_word == source
            assert all(observed['left_type'] == first.types[i][3]
                       for i, observed in enumerate(second.history[-1]['rows']))
            cached[active] = len(templates)
            templates.append({'active_native_row_ids': active, 'initial': asdict(initial),
                              'returned': asdict(second)})
        frames.append({'ports': ports, 'template': cached[active]})
        for port in ports:
            incidences[port] = incidences.get(port, 0) + 1
    if not frames:
        raise ValueError('source network has no arithmetic cells')
    primitive_frames = array('H')
    primitive_count = 0
    for wire, (op, a, b, required) in enumerate(struct.iter_unpack('<IIII', (directory / 'relation.ops').read_bytes())):
        if op not in (3, 4, 7):
            primitive_frames.append(65535)
            continue
        if op == 3:
            local_ports = (a, wire)
            primitive_rows = [[v, 3-v] for v in (1, 2)]
            first_binding, second_binding = (0, 0, 1), (1, 0, 1)
        else:
            local_ports = (a, b, wire)
            primitive_rows = [[x, y, 1 if ((x == 1 and y == 1) if op == 4 else (x == 1 or y == 1)) else 2]
                              for x in (1, 2) for y in (1, 2)]
            first_binding, second_binding = (0, 1, 2), (2, 0, 2)
        active = tuple(i for i, row in enumerate(primitive_rows)
                       if all(domains[p] & value for p, value in zip(local_ports, row))
                       and (op == 3 or a != b or row[0] == row[1]))
        key = (op, active)
        if key not in cached:
            initial = TypedJointRelation.arithmetic(source, range(len(local_ports)), [primitive_rows[i] for i in active], rules)
            first = initial.compose_into(rules, 22, 'join', *first_binding)
            returned = first.compose_into(rules, 26, 'meet', *second_binding)
            cached[key] = len(templates)
            templates.append({'kind': op, 'active_native_row_ids': active,
                              'initial': asdict(initial), 'returned': asdict(returned)})
        primitive_frames.append(cached[key])
        primitive_count += 1
    if len(templates) >= 65535:
        raise ValueError('native relation template index exhausted')
    if __import__('sys').byteorder != 'little':
        primitive_frames.byteswap()
    report = {'scope': 'complete source-bound joint-cell network under two explicit dialect composition bindings',
              'cells': len(frames), 'shared_wires': sum(n > 1 for n in incidences.values()),
              'primitive_relations': primitive_count,
              'distinct_execution_banks': len(templates),
              'joint_rows': sum(len(templates[f['template']]['active_native_row_ids']) for f in frames),
              'source_word': source,
              'route': [{'dialect': 22, 'operation': 'join', 'local_ports': [0, 1, 3]},
                        {'dialect': 26, 'operation': 'meet', 'local_ports': [3, 2, 4]}],
              'successive_operand_is_actual_return': True,
              'bindings': rules.bindings | {str((directory / p).resolve()): digest(directory / p) for p in names}}
    state = {'format': 'joint-dialect-network-v1', 'source_word': source,
             'prepared_directory': str(directory.resolve()), 'rules': rules.bindings,
             'templates': templates, 'frames': frames,
             'primitive_frame_index_le_u16_hex': primitive_frames.tobytes().hex(),
             'primitive_topology': str((directory / 'relation.ops').resolve()),
             'primitive_topology_sha256': digest(directory / 'relation.ops')}
    return report, state


def resume_network(directory, rules, parent_directory):
    parent_path = parent_directory / 'continuation.json'
    receipt = json.loads((parent_directory / 'receipt.json').read_text())
    if digest(parent_path) != receipt['continuation_sha256']:
        raise ValueError('returned joint network differs from its bound continuation')
    parent = json.loads(parent_path.read_text())
    if parent.get('format') != 'joint-dialect-network-v1':
        raise ValueError('continuation is not a joint dialect network')
    for path, expected in receipt['bindings'].items():
        if digest(path) != expected:
            raise ValueError(f'changed continuation source or native rule: {path}')
    source = (directory / 'source.imasm').read_text().strip()
    if parent['source_word'] != source or parent['rules'] != rules.bindings:
        raise ValueError('continuation changed its source or native rules')
    returned_templates = []
    changed_types = changed_banks = 0
    for template in parent['templates']:
        carried = TypedJointRelation(**template['returned'])
        width = len(carried.ports)
        left, right, output = (3, 4, 4) if width == 5 else (width-1, 0, width-1)
        # U67 receives the actual returned objects from U22 then U26.
        returned = carried.compose_into(rules, 67, 'tensor', left, right, output)
        for i, observed in enumerate(returned.history[-1]['rows']):
            assert list(observed['left_type']) == list(carried.types[i][left])
            assert observed['before'] == carried.rows[i][output]
        assert returned.origins == carried.origins
        changed_types += returned.types != carried.types
        changed_banks += any(row[output] != old[output] for row, old in zip(returned.rows, carried.rows))
        returned_templates.append({**template, 'returned': asdict(returned)})
    state = {'format': 'joint-dialect-network-v1', 'source_word': source,
             'prepared_directory': str(directory.resolve()), 'rules': rules.bindings,
             'templates': returned_templates,
             'topology_parent': {'path': str(parent_path.resolve()), 'sha256': digest(parent_path)}}
    report = {'scope': 'native U67 tensor on retained U22/U26 joint network continuation',
              'cells': receipt['cells'], 'primitive_relations': receipt['primitive_relations'],
              'distinct_execution_banks': len(returned_templates),
              'type_banks_changed': changed_types, 'terminal_banks_changed': changed_banks,
              'successive_operand_is_actual_return': True,
              'bindings': receipt['bindings'] | {str(parent_path.resolve()): digest(parent_path)}}
    return report, state


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('prepared_directory', type=Path)
    p.add_argument('output', type=Path)
    p.add_argument('--tables', required=True, type=Path)
    p.add_argument('--native-execution', required=True, type=Path)
    p.add_argument('--whole-network', action='store_true')
    p.add_argument('--resume-network', type=Path)
    args = p.parse_args()
    rules = NativeRules(args.tables, args.native_execution)
    report, state = (resume_network(args.prepared_directory, rules, args.resume_network)
                     if args.resume_network else
                     (network if args.whole_network else check)(args.prepared_directory, rules))
    args.output.mkdir(parents=True, exist_ok=False)
    (args.output / 'continuation.json').write_text(json.dumps(state, indent=2) + '\n')
    report['continuation_sha256'] = digest(args.output / 'continuation.json')
    report['instrument_sha256'] = digest(Path(__file__))
    report['relation_interpreter_sha256'] = digest(Path(__file__).with_name('relational_dialect.py'))
    (args.output / 'receipt.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({k: v for k, v in report.items() if k not in ('actions', 'bindings', 'source_word')}))


if __name__ == '__main__':
    main()
