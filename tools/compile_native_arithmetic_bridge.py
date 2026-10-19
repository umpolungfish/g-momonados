#!/usr/bin/env python3
"""Certify arithmetic returns of successive native tuple operations."""
import argparse
import json
from pathlib import Path
from relational_dialect import NativeRules, digest

ROUTE = [(22, 'join'), (26, 'join'), (67, 'tensor')]


def compile_bridge(tables, execution):
    rules = NativeRules(tables, execution)
    gates = {}
    for kind in (3, 4, 7):
        inputs = [(a, b) for a in (0, 1) for b in ((0,) if kind == 3 else (0, 1))]
        # Negation uses its definite native complement as the initial operand;
        # binary gates begin with their actual first operand.
        current = [rules.digit_types[1-a if kind == 3 else a] for a, b in inputs]
        stages = []
        for dialect, operation in ROUTE:
            before = current
            right = [rules.digit_types[1-a if kind == 3 else b] for a, b in inputs]
            composed = [rules.compose(dialect, operation, left, other)
                        for left, other in zip(before, right)]
            current = [value for value, _ in composed]
            expected = [(1-a if kind == 3 else a & b if kind == 4 else a | b)
                        for a, b in inputs]
            decoder = {}
            for value, output in zip(current, expected):
                key = ','.join(map(str, value))
                if key in decoder and decoder[key] != output:
                    raise ValueError('native operation merged distinct arithmetic results')
                decoder[key] = output
            stages.append({'dialect': dialect, 'operation': operation,
                           'before': before, 'right': right, 'images': current,
                           'conflicts': [evidence for _, evidence in composed],
                           'decoder': decoder})
        gates[str(kind)] = {'inputs': inputs, 'stages': stages, 'truth_outputs': expected}
    # Rule sensitivity: the very same grounded inputs and AND truth fibres
    # do not admit this return under canonical join.
    canonical = [rules.compose(0, 'join', rules.digit_types[a], rules.digit_types[b])[0]
                 for a in (0, 1) for b in (0, 1)]
    collision = next((i, j) for i in range(4) for j in range(i+1, 4)
                     if canonical[i] == canonical[j]
                     and ((i//2) & (i%2)) != ((j//2) & (j%2)))
    current={kind:gate['stages'][0]['before'] for kind,gate in gates.items()}
    seen={};phase=0
    while True:
        key=(phase%len(ROUTE),json.dumps(current,sort_keys=True))
        if key in seen:break
        seen[key]=phase
        dialect,operation=ROUTE[phase%len(ROUTE)]
        for kind,gate in gates.items():
            right=gate['stages'][0]['right']
            current[kind]=[rules.compose(dialect,operation,a,b)[0] for a,b in zip(current[kind],right)]
            decoder={}
            for image,bit in zip(current[kind],gate['truth_outputs']):
                if tuple(image) in decoder and decoder[tuple(image)]!=bit:
                    raise ValueError('continued native route lost arithmetic result')
                decoder[tuple(image)]=bit
        # Normalize tuple/list representation before cycle detection.
        current=json.loads(json.dumps(current));phase+=1
    axis_tables=[]
    for dialect,operation in ROUTE:
        op=rules.tables['operation_names'].index(operation)
        axis_tables.append(rules.tables['tables'][dialect]['axis_operations'][op])
    return {'format': 'native-arithmetic-return-v1', 'route': ROUTE,
            'axis_cards':[len(axis) for axis in rules.tables['axis_values']],
            'native_axis_tables':axis_tables,
            'cycle_start':seen[key], 'cycle_length':phase-seen[key],
            'rules': rules.bindings, 'gates': gates,
            'canonical_join_and_collision': list(collision),
            'native_full_tuple_checks': rules.checked_native_compositions,
            'compiler_sha256': digest(__file__)}


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--tables', type=Path, required=True)
    p.add_argument('--native-execution', type=Path, required=True)
    p.add_argument('output', type=Path)
    args = p.parse_args()
    result = compile_bridge(args.tables, args.native_execution)
    args.output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps({'route': result['route'],
                      'canonical_join_and_collision': result['canonical_join_and_collision'],
                      'sha256': digest(args.output)}))
