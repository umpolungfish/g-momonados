"""Spell source-derived operator exchanges; Gödel performs every number operation."""
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
SOURCE = sys.argv[1]
RECORD = ROOT / 'measurements' / ('godel_hex_slot_exchange_' + SOURCE + '.log')
EMPTY = '≻⋈∈⊤∋'
FILLED = '≻⋈∈⊥∋'
PAIRS = [('truth/falsity', 0, 1, '1'), ('truth/information', 0, 2, '3'),
         ('truth/fork', 0, 3, '7'), ('falsity/information', 1, 2, '2'),
         ('falsity/fork', 1, 3, '6'), ('information/fork', 2, 3, '4')]


def field(out, label):
    return next(line.split(None, 1)[1] for line in out.splitlines()
                if line.startswith(label + ' '))


with RECORD.open('a') as record:
    record.write('PROBE source=' + SOURCE + '\n')
    def run(args, allow_underflow=False):
        result = subprocess.run(args, cwd=ROOT, text=True,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        record.write('COMMAND ' + repr(args) + '\n' + result.stdout +
                     '\nEXIT ' + str(result.returncode) + '\n\n')
        if result.returncode:
            if allow_underflow and 'frame subtraction underflow' in result.stdout:
                return None
            raise RuntimeError(result.stdout)
        return result.stdout

    def decode(word):
        return run(['./godel', 'decode', word])

    def encode(value):
        return field(run(['./godel', 'encode', value]), 'word')

    def cells(word):
        body = word[1:-3]
        return [body[start:start + 5] for start in range(0, len(body), 5)]

    def spell(parts):
        return '⊢' + ''.join(parts) + '⊙⊡⊣'

    def paired(left, right):
        a, b = cells(left), cells(right)
        width = max(2, len(a), len(b))
        word = spell(a + [EMPTY] * (2 * width - len(a)) + b)
        value = field(decode(word), 'value')
        return value, width

    def difference(left, right):
        left_value = field(decode(left), 'value')
        right_value = field(decode(right), 'value')
        if right_value == '0':
            return left_value, encode(left_value)
        if left_value == '0':
            return right_value, encode(right_value)
        value, width = paired(left, right)
        args = ['./godel', 'frame-op', value, str(width), '0', 'sub',
                str(2 * width), '1']
        out = run(args, allow_underflow=True)
        if out is None:
            args = ['./godel', 'frame-op', value, str(2 * width), '1', 'sub',
                    str(width), '0']
            out = run(args)
        return field(out, 'result'), field(out, 'result-word')

    def operate(left, operation, right):
        value, width = paired(encode(left), encode(right))
        return run(['./godel', 'frame-op', value, str(width), '0', operation,
                    str(2 * width), '1'])

    raw = run(['./run_cmds.sh', 'trilattice_factor read ' + SOURCE])
    clean = re.sub(r'\x1b\[[0-9;]*m', '', raw)
    original = next(line.split(':', 1)[1].strip() for line in clean.splitlines()
                    if 'native word      :' in line)
    assert field(decode(original), 'value') == SOURCE
    source_cells = cells(original)
    source_cells += [EMPTY] * ((-len(source_cells)) % 4)

    for label, first, second, weight in PAIRS:
        record.write('OPERATOR EXCHANGE ' + label + '\n')
        changed = source_cells.copy()
        positive, negative = [], []
        for start in range(0, len(source_cells), 4):
            a, b = source_cells[start + first], source_cells[start + second]
            changed[start + first], changed[start + second] = b, a
            positive.extend([FILLED if a == FILLED and b == EMPTY else EMPTY,
                             EMPTY, EMPTY, EMPTY])
            negative.extend([FILLED if b == FILLED and a == EMPTY else EMPTY,
                             EMPTY, EMPTY, EMPTY])
        transformed_word = spell(changed)
        transformed = field(decode(transformed_word), 'value')
        # The fresh read returns the ordered hex motifs of the transformed source.
        run(['./run_cmds.sh', 'trilattice_factor read ' + transformed])
        placement, placement_word = difference(spell(positive), spell(negative))
        delta, _ = difference(original, transformed_word)
        product = run(['./godel', 'product', delta, weight, placement])
        assert 'relation.exact-product     PASS' in product
        if placement == '0':
            print(label, 'placement=0; transformed source retained')
            continue
        value, width = paired(original, placement_word)
        out = run(['./godel', 'frame-op', value, str(width), '0', 'divmod',
                   str(2 * width), '1'])
        quotient, remainder = field(out, 'result'), field(out, 'remainder')
        if remainder == '0' and placement != '1' and quotient != '1':
            closure = run(['./godel', 'product', SOURCE, placement, quotient])
            assert 'relation.exact-product     PASS' in closure
        print(label, 'placement=' + placement, 'quotient=' + quotient,
              'remainder=' + remainder)
        if remainder != '0' and quotient != '0':
            record.write('COUPLED OUTPUT AND REMAINDER\n')
            for component in (quotient, remainder):
                run(['./run_cmds.sh', 'trilattice_factor read ' + component])
            value, width = paired(encode(quotient), encode(remainder))
            coupled = run(['./godel', 'frame-op', value, str(width), '0',
                           'divmod', str(2 * width), '1'])
            print('output/remainder', 'quotient=' + field(coupled, 'result'),
                  'remainder=' + field(coupled, 'remainder'))
            reciprocal = operate(remainder, 'divmod', quotient)
            print('remainder/output', 'quotient=' + field(reciprocal, 'result'),
                  'remainder=' + field(reciprocal, 'remainder'))
            if field(reciprocal, 'remainder') == '0' and quotient != '1':
                collected = field(operate(placement, 'add',
                                          field(reciprocal, 'result')), 'result')
                closure = run(['./godel', 'product', SOURCE, quotient, collected])
                assert 'relation.exact-product     PASS' in closure
                print('output collection', quotient, collected, 'PASS')
            positioned = field(operate(placement, 'mul', quotient), 'result')
            whole = operate(positioned, 'divmod', remainder)
            print('positioned-output/remainder',
                  'quotient=' + field(whole, 'result'),
                  'remainder=' + field(whole, 'remainder'))
            if field(whole, 'remainder') == '0' and remainder != '1':
                collected = field(operate(field(whole, 'result'), 'add', '1'),
                                  'result')
                closure = run(['./godel', 'product', SOURCE, remainder, collected])
                assert 'relation.exact-product     PASS' in closure
                print('positioned remainder collection', remainder, collected,
                      'PASS')
            if field(coupled, 'remainder') == '0' and remainder != '1':
                collected = field(operate(placement, 'mul',
                                          field(coupled, 'result')), 'result')
                collected = field(operate(collected, 'add', '1'), 'result')
                closure = run(['./godel', 'product', SOURCE, remainder, collected])
                assert 'relation.exact-product     PASS' in closure
                print('remainder collection', remainder, collected, 'PASS')

print('record', RECORD)
