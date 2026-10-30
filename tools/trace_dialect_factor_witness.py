#!/usr/bin/env python3
"""Explain a completed first-hop promotion witness from independently replayed cells."""
import argparse
import hashlib
import json
import math
import mmap
from pathlib import Path
from verify_dialect_factor import bank_value, numeral


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('project', type=Path)
    p.add_argument('returned', type=Path)
    p.add_argument('verification', type=Path)
    p.add_argument('output', type=Path)
    args = p.parse_args()
    verified = json.loads(args.verification.read_text())
    with args.returned.open('rb') as stream:
        assert hashlib.file_digest(stream, 'sha256').hexdigest() == verified['receipt_sha256']
    tables = json.loads((args.project / 'dialect_tables.json').read_text())
    n = numeral(tables['source_word'])
    with args.returned.open('rb') as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as raw:
        marker = b',"members":['
        start = raw.find(marker) + len(marker)
        end = raw.find(b',{"dialect":', start)
        assert end > start
        member = json.loads(raw[start:end])
    step = member['ancestry'][0]
    radix = tables['crystal_radix'] * 256
    coefficients = [(b - a) * 256 for a, b in
                    zip(step['environment_after'], step['promoted_addresses'])]
    lift = 0
    for coefficient in coefficients:
        lift = lift * radix + coefficient
    shift = bank_value(step['promotion_shift'])
    assert lift % n == shift
    divisor = math.gcd(lift, n)
    probes = {key: {'value': bank_value(step[key]), 'gcd_with_source': math.gcd(bank_value(step[key]), n)}
              for key in ['frame_left', 'frame_right', 'frame_determinant', 'promotion_shift',
                          'support_value', 'support_derivative', 'support_divided', 'support_other_value']}
    result = {'question': __doc__, 'source_bits': n.bit_length(), 'dialect': member['dialect'],
              'operation': member['operation'], 'ruleset': step['ruleset'],
              'promotion_axis': step['promotion_axis'], 'tensor_radix': radix,
              'promotion_integer_coefficients': coefficients,
              'promotion_integer_lift': str(lift), 'promotion_modular_value': shift,
              'promotion_gcd_with_source': divisor, 'promotion_is_proper_divisor_witness': 1 < divisor < n,
              'first_hop_probes': probes,
              'boundary': 'terminal analysis of an independently replayed return; no reference factors used',
              'receipt_sha256': verified['receipt_sha256'],
              'instrument_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({k: v for k, v in result.items() if k not in
                      ['promotion_integer_lift', 'promotion_integer_coefficients']}, indent=2))


if __name__ == '__main__':
    main()
