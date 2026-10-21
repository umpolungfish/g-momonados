#!/usr/bin/env python3
"""Compare completed first sweeps from the same source and preparation."""
import argparse
import hashlib
import json
from pathlib import Path
from verify_dialect_factor import bank_value, numeral


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('anchor_project', type=Path)
    parser.add_argument('frame_project', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    old_path = args.anchor_project / 'instrument.json'
    new_path = args.frame_project / 'instrument.json'
    old, new = (json.loads(p.read_text()) for p in (old_path, new_path))
    assert not old.get('support_frame_point') and new['support_frame_point']
    assert old['source_word'] == new['source_word']
    assert (args.anchor_project / 'prepared.json').read_bytes() == (args.frame_project / 'prepared.json').read_bytes()
    n = numeral(new['source_word'])
    assert n.bit_length() >= 200
    width = n.bit_length() + 10
    prepared = json.loads((args.frame_project / 'prepared.json').read_text())
    p = bank_value(prepared['state'][3 * width:4 * width])
    q = bank_value(prepared['state'][4 * width:5 * width])
    changed_points = changed_words = changed_operands = coupled = checked = 0
    assert old['held'] == new['held'] == len(old['members']) == len(new['members'])
    for before, after in zip(old['members'], new['members']):
        assert (before['dialect'], before['operation']) == (after['dialect'], after['operation'])
        assert len(before['ancestry']) == len(after['ancestry']) == 1
        a, b = before['ancestry'][0], after['ancestry'][0]
        for key in ('labels', 'addresses', 'promoted_addresses', 'promotion_axis', 'promotion_shift',
                    'frame_left', 'frame_right', 'frame_determinant'):
            assert a[key] == b[key], 'first-sweep input or frame changed'
        op = after['operation']
        boundary = bank_value(b['frame_right' if op == 2 else 'frame_left'])
        shift = bank_value(b['promotion_shift'])
        assert bank_value(a['promotion_point']) == (p + shift) % n
        assert bank_value(b['promotion_point']) == (boundary + shift) % n
        # The selected row acts on both original arms. Perturb only the other
        # arm in this terminal counterfactual, retaining the same operator.
        matrix = [bank_value(w) for w in after['product_frame_state']['matrix']]
        row = matrix[2:] if op == 2 else matrix[:2]
        assert boundary == (row[0] * p + row[1] * q) % n
        coupled += ((row[0] * p + row[1] * ((q + 1) % n)) % n) != boundary
        changed_points += a['promotion_point'] != b['promotion_point']
        changed_words += a['support_corrected'] != b['support_corrected']
        changed_operands += before['product_frame_state']['anchor_left'] != after['product_frame_state']['anchor_left']
        assert before['state'][3 * width:5 * width] == after['state'][3 * width:5 * width] == prepared['state'][3 * width:5 * width]
        assert before['continuation'] == after['continuation'] == prepared['continuation']
        checked += 1
    assert changed_points and changed_words and changed_operands and coupled
    result = {
        'question': __doc__, 'source_bits': n.bit_length(), 'members_checked': checked,
        'changed_support_points': changed_points, 'changed_corrected_words': changed_words,
        'changed_selected_operands': changed_operands,
        'frame_points_sensitive_to_other_arm': coupled,
        'same_source_and_prepared_banks': True, 'Y_continuation_preserved': True,
        'boundary': 'completed compilation instruments; counterfactual changes no executing state',
        'bindings': {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in (old_path, new_path, Path(__file__))},
    }
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
