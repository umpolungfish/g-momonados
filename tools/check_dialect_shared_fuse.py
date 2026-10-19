#!/usr/bin/env python3
"""Exercise dialect-frame fusion on the persisted source's actual carry edges."""
import argparse
from collections import Counter
import json
from pathlib import Path
import struct
from relational_dialect import TypedJointRelation, digest


def load_bound(directory):
    path = directory / 'continuation.json'
    receipt = json.loads((directory / 'receipt.json').read_text())
    if digest(path) != receipt['continuation_sha256']:
        raise ValueError('continuation hash mismatch')
    state = json.loads(path.read_text())
    for source, expected in receipt['bindings'].items():
        if digest(source) != expected:
            raise ValueError(f'changed source binding: {source}')
    return state, path


def check(directory):
    state, path = load_bound(directory)
    topology = state
    while 'frames' not in topology:
        parent = topology['topology_parent']
        if digest(parent['path']) != parent['sha256']:
            raise ValueError('changed topology parent')
        topology = json.loads(Path(parent['path']).read_text())
    if state['source_word'] != topology['source_word']:
        raise ValueError('topology belongs to a different source')
    relations = [TypedJointRelation(**t['returned']) for t in state['templates']]
    for relation, definition in zip(relations, state['templates']):
        for row in relation.origins:
            values = [v == 1 for v in row]
            kind = definition.get('kind')
            if kind == 3:
                assert values[1] == (not values[0])
            elif kind == 4:
                assert values[2] == (values[0] and values[1])
            elif kind == 7:
                assert values[2] == (values[0] or values[1])
            else:
                assert len(values) == 5
                assert sum(values[:3]) == values[3] + 2*values[4]
    readers = {}
    for index, frame in enumerate(topology['frames']):
        for wire in frame['ports'][:3]:
            readers.setdefault(wire, []).append(index)
    classes = Counter()
    for frame in topology['frames']:
        for index in readers.get(frame['ports'][4], []):
            other = topology['frames'][index]
            shared = tuple((i, j) for i, a in enumerate(frame['ports'])
                           for j, b in enumerate(other['ports']) if a == b)
            classes[(frame['template'], other['template'], shared)] += 1
    carry_edges = sum(classes.values())
    # Follow producer-to-consumer edges in the prepared primitive network.
    # Constants and factor variables retain their prepared boundary supports;
    # they do not acquire invented predicate frames.
    raw = Path(topology['primitive_topology']).read_bytes()
    if digest(topology['primitive_topology']) != topology['primitive_topology_sha256']:
        raise ValueError('changed primitive topology')
    frame_ids = bytes.fromhex(topology['primitive_frame_index_le_u16_hex'])
    if len(frame_ids) * 8 != len(raw):
        raise ValueError('primitive frame index has the wrong length')
    def ports(wire):
        op, a, b, _ = struct.unpack_from('<IIII', raw, wire*16)
        return (a, wire) if op == 3 else (a, b, wire)
    def template(wire):
        return struct.unpack_from('<H', frame_ids, wire*2)[0]
    boundary_edges = 0
    for wire, (op, a, b, _) in enumerate(struct.iter_unpack('<IIII', raw)):
        target = template(wire)
        if target == 65535:
            continue
        downstream = ports(wire)
        for source in set(downstream[:-1]):
            producer = template(source)
            if producer == 65535:
                boundary_edges += 1
                continue
            upstream = ports(source)
            shared = tuple((i, j) for i, x in enumerate(upstream)
                           for j, y in enumerate(downstream) if x == y)
            classes[(producer, target, shared)] += 1
    records, tests = [], 0
    unequal_images = 0
    for (a, b, shared), count in sorted(classes.items()):
        left, right = relations[a], relations[b]
        widths = (len(left.ports), len(right.ports))
        domains = [([3]*widths[0], [3]*widths[1])]
        for side in range(2):
            for port in range(widths[side]):
                for value in (1, 2):
                    pair = [[3]*widths[0], [3]*widths[1]]
                    pair[side][port] = value
                    domains.append(pair)
        for ld, rd in domains:
            result = left.fuse_shared(right, shared, ld, rd)
            # Independent source assignment join, with both arithmetic cells
            # evaluated before enforcing the actual common-wire bindings.
            expected = []
            for i, lr in enumerate(left.origins):
                lv = [int(v == 1) for v in lr]
                if len(lv) == 5:
                    assert sum(lv[:3]) == lv[3] + 2*lv[4]
                for j, rr in enumerate(right.origins):
                    rv = [int(v == 1) for v in rr]
                    if len(rv) == 5:
                        assert sum(rv[:3]) == rv[3] + 2*rv[4]
                    if (all(v & d for v, d in zip(lr, ld)) and
                        all(v & d for v, d in zip(rr, rd)) and
                        all(lr[x] == rr[y] for x, y in shared)):
                        expected.append((i, j))
            assert result['pairs'] == expected
            assert result['contradiction'] == (not expected)
            for side, relation in enumerate((left, right)):
                projection = [0]*len(relation.ports)
                for pair in expected:
                    for k, value in enumerate(relation.origins[pair[side]]):
                        projection[k] |= value
                assert result['left_supports' if side == 0 else 'right_supports'] == projection
            for (i, j), witness in zip(result['pairs'], result['witnesses']):
                assert witness['left'] == left.observation(i, left.ports)
                assert witness['right'] == right.observation(j, right.ports)
            tests += 1
        result = left.fuse_shared(right, shared, [3]*widths[0], [3]*widths[1])
        for i, j in result['pairs']:
            unequal_images += sum(left.observation(i, [x]) != right.observation(j, [y])
                                  for x, y in shared)
        records.append({'left_template': a, 'right_template': b,
                        'shared': shared, 'source_edge_count': count,
                        'admitted_pairs': len(result['pairs']),
                        'returned_fusion': result})
    return {'parent': str(path), 'parent_sha256': digest(path),
            'carry_edges': carry_edges,
            'primitive_edges': sum(classes.values()) - carry_edges,
            'primitive_boundary_edges': boundary_edges, 'classes': records,
            'checks': tests, 'equal_source_values_with_unequal_images': unequal_images}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('continuation', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    report = check(args.continuation)
    report['instrument_sha256'] = digest(__file__)
    report['interpreter_sha256'] = digest(Path(__file__).with_name('relational_dialect.py'))
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({k: v for k, v in report.items() if k != 'classes'}))
