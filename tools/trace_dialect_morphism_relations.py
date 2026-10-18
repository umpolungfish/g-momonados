#!/usr/bin/env python3
"""Trace exact unary morphism compositions on shared primitive/conflict states."""
import argparse
import hashlib
import itertools
import json
from pathlib import Path


def compose(left, right):
    return tuple(left[right[x]] for x in range(len(right)))


def trace(tables):
    axes = []
    for axis, values in enumerate(tables['axis_values']):
        card = len(values)
        actions = {}
        for dialect in tables['tables']:
            for operation in range(3):
                table = dialect['axis_operations'][operation][axis]
                for side in range(2):
                    for anchor in range(card):
                        action = []
                        for conflict in range(2):
                            for x in range(card):
                                a, b = (anchor, x) if side == 0 else (x, anchor)
                                value, evidence = table[a * card + b]
                                action.append(value + card * (conflict | bool(evidence)))
                        actions.setdefault(tuple(action), []).append(
                            [dialect['id'], operation, side, anchor])
        maps = list(actions)
        canonical = [i for i, action in enumerate(maps)
                     if any(label[0] == 0 for label in actions[action])]
        products = {}
        product_indices = []
        commuting = []
        witnesses = []
        for i, left in enumerate(maps):
            row = []
            for j, right in enumerate(maps):
                product = compose(left, right)
                if product not in products:
                    products[product] = len(products)
                row.append(products[product])
                if j > i:
                    reverse = compose(right, left)
                    if product == reverse:
                        commuting.append([i, j])
                    else:
                        x = next(x for x in range(2 * card) if product[x] != reverse[x])
                        witnesses.append([i, j, x, product[x], reverse[x]])
            product_indices.append(row)
        # A change of coordinates is a separate question from commutation.
        # Exhaust every permutation of this finite primitive domain, preserving
        # conflict evidence and all three binary operations simultaneously.
        isomorphisms = []
        base = tables['tables'][0]['axis_operations']
        for dialect in tables['tables']:
            admitted = []
            target = dialect['axis_operations']
            for permutation in itertools.permutations(range(card)):
                valid = True
                for op in range(3):
                    for a in range(card):
                        for b in range(card):
                            value, conflict = base[op][axis][a * card + b]
                            other, other_conflict = target[op][axis][permutation[a] * card + permutation[b]]
                            if permutation[value] != other or conflict != other_conflict:
                                valid = False
                                break
                        if not valid:
                            break
                    if not valid:
                        break
                if valid:
                    admitted.append(list(permutation))
            isomorphisms.append({'dialect': dialect['id'], 'coordinate_permutations': admitted})
        axes.append({
            'axis': axis, 'primitive_values': values,
            'state_rule': 'index=value+cardinality*held_conflict; conflict is accumulated by OR',
            'actions': [{'map': list(action), 'definitions': actions[action]} for action in maps],
            'canonical_action_classes': canonical,
            'composition_maps': [list(action) for action in products],
            'composition_indices': product_indices,
            'commuting_unordered_pairs': commuting,
            'noncommuting_witnesses': witnesses,
            'canonical_simultaneous_binary_isomorphisms': isomorphisms,
        })
    return axes


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('tables', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    raw = args.tables.read_bytes()
    tables = json.loads(raw)
    axes = trace(tables)
    result = {'question': __doc__, 'tables_sha256': hashlib.sha256(raw).hexdigest(),
              'instrument_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              'domain': 'every exported dialect, operation, side, anchor, primitive value and held-conflict state; every pair of elementary unary actions',
              'composition_order': 'left after right; no canonical return inserted',
              'axes': axes}
    args.output.write_text(json.dumps(result, separators=(',', ':')) + '\n')
    print(json.dumps([{'axis': a['axis'], 'elementary_classes': len(a['actions']),
                      'composite_classes': len(a['composition_maps']),
                      'commuting_pairs': len(a['commuting_unordered_pairs']),
                      'noncommuting_pairs': len(a['noncommuting_witnesses']),
                      'dialects_isomorphic_to_canonical': sum(bool(d['coordinate_permutations']) for d in a['canonical_simultaneous_binary_isomorphisms'])}
                     for a in axes], indent=2))


if __name__ == '__main__':
    main()
