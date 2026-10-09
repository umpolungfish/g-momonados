"""Certify source common components with the six exact hex exchange weights."""
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

with (ROOT / 'measurements' / ('godel_hex_exchange_weight_' + record_name(SOURCE) + '.log')).open('a') as record:
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

    run(['python3', 'measurements/godel_hex_ordered_word_probe.py', SOURCE])
    read(SOURCE)
    for first, second in ((0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)):
        lower = decode([EMPTY] * first + [FILLED])
        upper = decode([EMPTY] * second + [FILLED])
        weight = field(operate(upper, 'sub', lower), 'result')
        read(weight)
        if weight == '1':
            common = '1'
        else:
            returned = operate(SOURCE, 'divmod', weight)
            quotient, remainder = field(returned, 'result'), field(returned, 'remainder')
            placed = field(operate(weight, 'mul', quotient), 'result')
            assert field(operate(placed, 'add', remainder), 'result') == SOURCE
            record.write('SOURCE / EXCHANGE WEIGHT slots=' + str((first, second)) +
                         ' weight=' + weight + ' remainder=' + remainder + '\n')
            # Reduce only the fixed operator weight and its verified remainder.
            # These weights are 1, 2, 3, 4, 6, 7, independent of source size.
            a, b = weight, remainder
            while b != '0':
                divided = operate(a, 'divmod', b)
                a, b = b, field(divided, 'remainder')
            common = a
        record.write('EXCHANGE WEIGHT COMMON COMPONENT weight=' + weight +
                     ' source-common=' + common + ' exact=PASS\n')
        print(SOURCE, 'slots', first, second, 'weight', weight, 'common', common)
        if common != '1':
            divided = operate(SOURCE, 'divmod', common)
            assert field(divided, 'remainder') == '0'
            product(SOURCE, common, field(divided, 'result'))
