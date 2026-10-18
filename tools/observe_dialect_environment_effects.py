#!/usr/bin/env python3
"""Read carried environment effects and multi-hop edges from a verified return."""
import argparse
import hashlib
import json
import math
import mmap
from pathlib import Path
from verify_dialect_factor import numeral


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project', type=Path)
    parser.add_argument('returned', type=Path)
    parser.add_argument('verification', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    verified = json.loads(args.verification.read_text())
    with args.returned.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    assert digest == verified['receipt_sha256'], 'return differs from its independent replay'
    tables = json.loads((args.project / 'dialect_tables.json').read_text())
    n = numeral(tables['source_word'])
    radix = tables['crystal_radix'] * 256
    cards = list(map(len, tables['axis_values']))
    strides = [math.prod(cards[i + 1:]) for i in range(12)]
    steps = coordinate_changes = image_changes = edges = noncanonical_edges = 0
    rule_coordinate_changes = rule_image_changes = 0
    environments = set()

    def image(coordinates, labels):
        value = 0
        for address, label in zip(coordinates, labels):
            value = (value * radix + address * 256 + label) % n
        return value

    with args.returned.open('rb') as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as raw:
        header_end = raw.find(b',"parent_return_gzip_hex"')
        header = json.loads(raw[:header_end] + b'}')
        assert header['environment_carry'] and header['keep_fibres']
        assert header['source_word'] == tables['source_word']
        marker = b',"members":['
        position = raw.find(marker)
        assert position >= 0
        position += len(marker)
        while position < raw.size() - 3:
            next_position = raw.find(b',{"dialect":', position)
            end = next_position if next_position >= 0 else raw.rfind(b']}')
            member = json.loads(raw[position:end])
            previous = None
            for step in member['ancestry']:
                current = step['ruleset']
                environments.add(current)
                if previous is not None:
                    edges += 1
                    noncanonical_edges += previous != 0 and current != 0
                previous = current
                coordinate_changes += sum(a != b for a, b in zip(step['addresses'], step['environment_after']))
                plain = step['addresses']
                carried = step['environment_after']
                canonical = list(carried)
                if step['environment_before']:
                    canonical = []
                    for left, right in zip(step['environment_before'], step['addresses']):
                        address = 0
                        for axis, (card, stride) in enumerate(zip(cards, strides)):
                            cell = tables['tables'][0]['axis_operations'][step.get('environment_operation',member['operation'])][axis][(left // stride % card) * card + right // stride % card]
                            address += cell[0] * stride
                        canonical.append(address)
                    rule_coordinate_changes += sum(a != b for a, b in zip(canonical, carried))
                if header['promotion_return']:
                    axis = step['promotion_axis']
                    card, stride = cards[axis], strides[axis]
                    target = tables['promotion_target_indices'][axis]
                    plain = [a - (a // stride % card) * stride + target * stride for a in plain]
                    canonical = [a - (a // stride % card) * stride + target * stride for a in canonical]
                    carried = step['promoted_addresses']
                image_changes += image(plain, step['labels']) != image(carried, step['labels'])
                rule_image_changes += image(canonical, step['labels']) != image(carried, step['labels'])
                steps += 1
            if next_position < 0:
                break
            position = next_position + 1
    assert steps == verified['environment_steps']
    assert coordinate_changes == verified['environment_changed_coordinates']
    result = {
        'question': __doc__, 'source_bits': n.bit_length(), 'environment_steps': steps,
        'changed_type_coordinates': coordinate_changes, 'changed_register_images': image_changes,
        'environment_rule_coordinate_changes': rule_coordinate_changes,
        'environment_rule_register_changes': rule_image_changes,
        'adjacent_route_edges': edges, 'noncanonical_to_noncanonical_edges': noncanonical_edges,
        'visited_environments': sorted(environments),
        'comparison': 'same labels and promotion, omit accumulated environment action in a terminal counterfactual',
        'ruleset_comparison': 'same full input tuples, labels and promotion; replace only the acting environment algebra with canonical algebra',
        'boundary': 'completed independently replayed return; no executing state read or changed',
        'receipt_sha256': digest,
        'observer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    }
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
