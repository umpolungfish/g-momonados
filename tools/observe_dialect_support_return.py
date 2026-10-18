#!/usr/bin/env python3
"""Read the source-support divided difference between two returned dialect points."""
import argparse
import hashlib
import json
import math
import mmap
from pathlib import Path
from verify_dialect_factor import bank_value, numeral


def divided_support(n, x, y):
    px = py = divided = 0
    for i in range(n.bit_length() - 1, -1, -1):
        divided = (y * divided + px) % n
        bit = (n >> i) & 1
        px = (x * px + bit) % n
        py = (y * py + bit) % n
    assert (px - py - (x - y) * divided) % n == 0
    return px, py, divided


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project', type=Path)
    parser.add_argument('returned', type=Path)
    parser.add_argument('verification', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    report = json.loads(args.verification.read_text())
    with args.returned.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    assert digest == report['receipt_sha256'], 'return verification binding changed'
    tables = json.loads((args.project / 'dialect_tables.json').read_text())
    n = numeral(tables['source_word'])
    assert n.bit_length() >= 200
    checked = units = zero_shifts = 0
    factors = []
    with args.returned.open('rb') as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as raw:
        stop = raw.find(b',"parent_return_sha256"')
        header = json.loads(raw[:stop] + b'}')
        assert header['source_word'] == tables['source_word']
        assert header['terminal_only_source_readout'], 'requires a completed terminal-boundary return'
        marker = b',"members":['
        position = raw.find(marker) + len(marker)
        while position < raw.size() - 3:
            next_position = raw.find(b',{"dialect":', position)
            end = next_position if next_position >= 0 else raw.rfind(b']}')
            member = json.loads(raw[position:end])
            if member['ancestry']:
                last = member['ancestry'][-1]
                x = bank_value(last['promotion_point'])
                shift = bank_value(last['promotion_shift'])
                y = (x - shift) % n
                px, py, divided = divided_support(n, x, y)
                assert (y + shift) % n == x
                checked += 1
                zero_shifts += shift == 0
                units += math.gcd(divided, n) == 1
                for kind, witness in [('divided', divided), ('shift', shift)]:
                    divisor = math.gcd(witness, n)
                    if 1 < divisor < n:
                        quotient, remainder = divmod(n, divisor)
                        assert remainder == 0 and divisor * quotient == n
                        factors.append({'dialect': member['dialect'], 'operation': member['operation'],
                                        'kind': kind, 'witness': str(witness),
                                        'divisor': str(divisor), 'quotient': str(quotient)})
            if next_position < 0:
                break
            position = next_position + 1
    result = {'question': __doc__, 'source_bits': n.bit_length(), 'returned_members': report['members'],
              'returned_point_pairs_checked': checked, 'unit_divided_values': units,
              'zero_promotion_shifts': zero_shifts, 'proper_divisor_pairs': factors,
              'return_sha256': digest, 'execution_boundary': 'completed return only; input file unchanged'}
    with args.returned.open('rb') as stream:
        assert hashlib.file_digest(stream, 'sha256').hexdigest() == digest
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
