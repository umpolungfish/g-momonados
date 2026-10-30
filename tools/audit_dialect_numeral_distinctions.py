#!/usr/bin/env python3
"""Check whether exported type inputs retain elementary multiplication distinctions."""
import argparse
import hashlib
import json
from pathlib import Path


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('tables', type=Path)
    p.add_argument('output', type=Path)
    args = p.parse_args()
    raw = args.tables.read_bytes()
    tables = json.loads(raw)
    results = []
    for dialect in tables['tables']:
        fibres = {}
        for numeral, address in enumerate(dialect['input_addresses']):
            fibres.setdefault(address, []).append(numeral)
        products = {}
        collisions = []
        for a in range(16):
            for b in range(16):
                key = (dialect['input_addresses'][a], dialect['input_addresses'][b])
                output = (a * b % 16, a * b // 16)
                if key in products and products[key][2] != output:
                    if not collisions:
                        previous = products[key]
                        collisions.append({'first_inputs': list(previous[:2]),
                                           'first_product_cells': list(previous[2]),
                                           'second_inputs': [a, b],
                                           'second_product_cells': list(output),
                                           'shared_type_inputs': list(key)})
                else:
                    products[key] = (a, b, output)
        results.append({'dialect': dialect['id'], 'input_fibres': list(fibres.values()),
                        'distinct_input_types': len(fibres),
                        'distinct_binary_output_types': [len({cell[0] for cell in op})
                                                        for op in dialect['operations']],
                        'multiplication_not_determined_by_types_alone': bool(collisions),
                        'counterexample': collisions})
    result = {'question': __doc__, 'tables_sha256': hashlib.sha256(raw).hexdigest(),
              'domain': 'all 256 ordered pairs of canonical hexadecimal cell values; low product and carry',
              'scope': 'exported type classification alone; original labels remain retained in the full membrane',
              'dialects': results}
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({'canonical': results[0], 'dialects_with_16_distinct_inputs': sum(
        r['distinct_input_types'] == 16 for r in results),
        'dialects_with_multiplication_type_collisions': sum(
        r['multiplication_not_determined_by_types_alone'] for r in results)}, indent=2))


if __name__ == '__main__':
    main()
