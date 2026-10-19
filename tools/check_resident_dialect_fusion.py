#!/usr/bin/env python3
"""Check continued native dialect action on a resident correlated frame pair."""
import argparse
from dataclasses import asdict
import json
from pathlib import Path
from relational_dialect import NativeRules, TypedJointRelation, FusedJointRelation, digest
from check_dialect_shared_fuse import load_bound


def check(parent, rules, output):
    state, path = load_bound(parent)
    topology = state
    while 'frames' not in topology:
        pointer = topology['topology_parent']
        if digest(pointer['path']) != pointer['sha256']:
            raise ValueError('changed topology')
        topology = json.loads(Path(pointer['path']).read_text())
    if state['rules'] != rules.bindings:
        raise ValueError('resident rules differ from executed rules')
    readers = {}
    for frame in topology['frames']:
        for wire in frame['ports'][:3]:
            readers.setdefault(wire, []).append(frame)
    # Select an actual nontrivial source edge, without knowing any factors.
    selected = None
    for first in topology['frames']:
        for second in readers.get(first['ports'][4], []):
            left = TypedJointRelation(**state['templates'][first['template']]['returned'])
            right = TypedJointRelation(**state['templates'][second['template']]['returned'])
            shared = [(i, j) for i, a in enumerate(first['ports'])
                      for j, b in enumerate(second['ports']) if a == b]
            resident = FusedJointRelation.connect(left, right, shared, [3]*5, [3]*5)
            if len(resident.pairs) > 2:
                selected = (first, second, resident)
                break
        if selected:
            break
    if selected is None:
        raise ValueError('no nontrivial carried pair')
    first, second, resident = selected
    # A diagnostic observation removes one joint alternative while leaving
    # both projected support banks unchanged. This is a retention control,
    # not a source-derived deduction.
    narrowed = None
    for removed in resident.pairs:
        allowed = [(resident.left.observation(i, resident.left.ports),
                    resident.right.observation(j, resident.right.ports))
                   for i, j in resident.pairs if (i, j) != removed]
        candidate = resident.restrict(resident.left.ports, resident.right.ports, allowed)
        if len(candidate.pairs) < len(resident.pairs) and candidate.supports() == resident.supports():
            narrowed = candidate
            break
    if narrowed is None:
        raise ValueError('selected pair lacks a correlation-only restriction')
    restored = FusedJointRelation(**json.loads(json.dumps(asdict(narrowed))))
    assert restored.pairs == narrowed.pairs
    assert restored.supports() == narrowed.supports()
    actions = []
    for dialect in range(len(rules.tables['tables'])):
        for operation in rules.tables['operation_names']:
            changed = restored.compose_into('left', rules, dialect, operation, 0, 1, 3)
            assert changed.pairs == restored.pairs
            assert changed.supports() == restored.supports()
            assert changed.right == restored.right
            assert changed.left.origins == restored.left.origins
            observations = [(changed.left.observation(i, changed.left.ports),
                             changed.right.observation(j, changed.right.ports))
                            for i, j in resident.pairs]
            reread = changed.restrict(changed.left.ports, changed.right.ports, observations)
            assert reread.pairs == narrowed.pairs, 'observation resurrected a removed pair'
            actions.append({'dialect': dialect, 'operation': operation,
                            'types_changed': changed.left.types != restored.left.types,
                            'values_changed': changed.left.rows != restored.left.rows})
    # Continue one actual resident state through two more environments.
    returned = restored.compose_into('left', rules, 22, 'join', 0, 1, 3)
    returned = returned.compose_into('right', rules, 67, 'tensor', 3, 4, 4)
    assert returned.pairs == narrowed.pairs
    empty = returned.restrict(returned.left.ports, returned.right.ports, [])
    assert empty.pairs == [] and empty.supports() == ([0]*5, [0]*5)
    assert empty.compose_into('left', rules, 26, 'meet', 0, 1, 3).pairs == []
    output.mkdir(exist_ok=False)
    result_path = output / 'continuation.json'
    result_path.write_text(json.dumps({'format': 'resident-dialect-fusion-v1',
                                    'parent': str(path), 'parent_sha256': digest(path),
                                    'source_frames': [first, second],
                                    'observation_scope': 'diagnostic correlation-retention control',
                                    'relation': asdict(returned)}, indent=2)+'\n')
    report = {'parent_sha256': digest(path), 'source_frames': [first, second],
              'initial_pairs': len(resident.pairs), 'retained_pairs': len(returned.pairs),
              'projected_supports_unchanged': returned.supports() == resident.supports(),
              'native_actions': actions, 'empty_successor_preserved': True,
              'continuation_sha256': digest(result_path),
              'instrument_sha256': digest(__file__),
              'interpreter_sha256': digest(Path(__file__).with_name('relational_dialect.py'))}
    (output / 'receipt.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps({k: v for k, v in report.items() if k != 'native_actions'}))


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('parent', type=Path)
    p.add_argument('output', type=Path)
    p.add_argument('--tables', required=True, type=Path)
    p.add_argument('--native-execution', required=True, type=Path)
    a = p.parse_args()
    check(a.parent, NativeRules(a.tables, a.native_execution), a.output)
