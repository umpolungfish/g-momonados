"""Retain and collect unequal positioned lanes through their shared payload."""
from godel_hex_record_name import record_name
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'

def field(out, label):
    return next(line.split(None, 1)[1] for line in out.splitlines()
                if line.startswith(label + ' '))

def cells(word):
    return [word[1:-3][i:i + 5] for i in range(0, len(word[1:-3]), 5)]

SOURCE = sys.argv[1] if len(sys.argv) > 1 else '213'

with (ROOT / 'measurements' / ('godel_hex_lane_residual_' + record_name(SOURCE) + '.log')).open('a') as record:
    def run(args, allow_underflow=False):
        out = subprocess.run(args, cwd=ROOT, text=True,
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        record.write('COMMAND ' + repr(args) + '\n' + out.stdout +
                     '\nEXIT ' + str(out.returncode) + '\n\n')
        if out.returncode:
            if allow_underflow and 'frame subtraction underflow' in out.stdout:
                return None
            raise RuntimeError(out.stdout)
        return out.stdout

    def decode(parts):
        return field(run(['./godel', 'decode', '⊢' + ''.join(parts) + '⊙⊡⊣']), 'value')

    def encode(value):
        return cells(field(run(['./godel', 'encode', value]), 'word'))

    def read(value):
        out = run(['./run_cmds.sh', 'trilattice_factor read ' + value])
        clean = re.sub(r'\x1b\[[0-9;]*m', '', out)
        return cells(next(line.split(':', 1)[1].strip()
                          for line in clean.splitlines()
                          if 'native word      :' in line))

    def operate(a, op, b):
        left, right = encode(a), encode(b)
        if b == '0' and op in {'add', 'sub'}:
            record.write('ZERO-RIGHT IDENTITY operation=' + op + ' result=' + a + '\n')
            return 'result ' + a + '\n'
        width = max(2, len(left), len(right))
        composed = decode(left + [EMPTY] * (2 * width - len(left)) + right)
        return run(['./godel', 'frame-op', composed, str(width), '0', op,
                    str(2 * width), '1'], allow_underflow=(op == 'sub'))

    def product(n, a, b):
        out = run(['./godel', 'product', n, a, b])
        assert 'relation.exact-product     PASS' in out

    read(SOURCE)
    separated = run(['./godel', 'unbraid', SOURCE])
    gamma = cells(field(separated, 'Γ.word'))
    lam = cells(field(separated, 'Λ.word'))
    a = decode([part for cell in gamma for part in (cell, EMPTY)])
    b = decode([part for cell in lam for part in (cell, EMPTY)])
    two_b = field(operate(b, 'mul', '2'), 'result')
    assert field(operate(a, 'add', two_b), 'result') == SOURCE
    pair_payload = decode([FILLED, FILLED])
    delta = operate(a, 'sub', b)
    sign = '+'
    if delta is None:
        delta = operate(b, 'sub', a)
        sign = '-'
    magnitude = field(delta, 'result')
    read(magnitude)
    counts = []
    for lane in (gamma, lam):
        count = '0'
        for cell in lane:
            if cell == FILLED:
                count = '1' if count == '0' else field(operate(count, 'add', '1'), 'result')
        counts.append(count)
    folded = operate(counts[0], 'sub', counts[1])
    fold_sign = '+'
    if folded is None:
        folded = operate(counts[1], 'sub', counts[0])
        fold_sign = '-'
    fold_value = field(folded, 'result')
    folded_return = operate(fold_value, 'divmod', pair_payload)
    record.write('LANE FOLD counts=' + repr(counts) + ' signed=' + fold_sign +
                 fold_value + ' remainder=' + field(folded_return, 'remainder') + '\n')
    record.write('SOURCE LANE RETURN n=' + SOURCE + ' B=' + b +
                 ' signed-difference=' + sign + magnitude + '\n')
    if field(folded_return, 'remainder') != '0':
        print(SOURCE, 'lane residual retained; signed fold', fold_sign + fold_value,
              'remainder', field(folded_return, 'remainder'))
        sys.exit(0)
    returned = operate(magnitude, 'divmod', pair_payload)
    assert field(returned, 'remainder') == '0'
    coefficient = field(returned, 'result')
    product(magnitude, pair_payload, coefficient)
    cofactor = b if coefficient == '0' else field(
        operate(b, 'add' if sign == '+' else 'sub', coefficient), 'result')
    product(SOURCE, pair_payload, cofactor)
    read(cofactor)
    proper = cofactor not in {'0', '1', SOURCE} and pair_payload != SOURCE
    record.write('SOURCE CERTIFICATE exact=PASS proper=' + str(proper) + '\n')
    print(SOURCE, 'lane residual collection', pair_payload, cofactor,
          'proper-factor PASS' if proper else 'unit return')

