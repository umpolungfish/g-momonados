"""Certify common components of source-selected positioned hex exchange differences."""
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

with (ROOT / 'measurements' / ('godel_hex_exchange_placement_' + SOURCE + '.log')).open('a') as record:
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
    parts = read(SOURCE)
    assert parts[0] == FILLED, 'This certificate requires an odd source'
    parts += [EMPTY] * ((-len(parts)) % 4)
    def odd_tail(value):
        word = encode(value)
        shift = 0
        for cell in word:
            if cell != EMPTY:
                break
            shift += 1
        if value == '0' or shift == 0:
            return value
        result = decode(word[shift:])
        scale = decode([EMPTY] * shift + [FILLED])
        product(value, scale, result)
        record.write('REMOVE INITIAL EMPTY CELLS shift=' + str(shift) +
                     ' odd-tail=' + result + '\n')
        return result
    def common_component(a, b):
        if a == '0':
            return b
        if b == '0':
            return a
        a, b = odd_tail(a), odd_tail(b)
        while a != b:
            previous_width = len(encode(a)) + len(encode(b))
            difference = operate(a, 'sub', b)
            if difference is None:
                difference = operate(b, 'sub', a)
                b = odd_tail(field(difference, 'result'))
            else:
                a = odd_tail(field(difference, 'result'))
            assert len(encode(a)) + len(encode(b)) < previous_width
        return a
    for first, second in ((0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)):
        positive, negative = [], []
        changed = parts.copy()
        for start in range(0, len(parts), 4):
            a, b = parts[start + first], parts[start + second]
            changed[start + first], changed[start + second] = b, a
            positive.extend([FILLED if a == FILLED and b == EMPTY else EMPTY, EMPTY, EMPTY, EMPTY])
            negative.extend([FILLED if b == FILLED and a == EMPTY else EMPTY, EMPTY, EMPTY, EMPTY])
        pos, neg = decode(positive), decode(negative)
        delta = operate(pos, 'sub', neg)
        sign = '+'
        if delta is None:
            delta = operate(neg, 'sub', pos)
            sign = '-'
        placement = field(delta, 'result')
        record.write('SOURCE SELECTED PLACEMENT slots=' + str((first, second)) +
                     ' signed=' + sign + placement + '\n')
        if placement == '0':
            record.write('UNCHANGED EXCHANGE: whole source common component\n')
            continue
        exchanged = decode(changed)
        magnitude = operate(exchanged, 'sub', SOURCE) if sign == '+' else operate(SOURCE, 'sub', exchanged)
        assert magnitude is not None
        lower, upper = decode([EMPTY] * first + [FILLED]), decode([EMPTY] * second + [FILLED])
        weight = field(operate(upper, 'sub', lower), 'result')
        product(field(magnitude, 'result'), weight, placement)
        common = common_component(SOURCE, placement)
        record.write('PLACEMENT COMMON COMPONENT slots=' + str((first, second)) +
                     ' common=' + common + ' exact=PASS\n')
        print(SOURCE, 'slots', first, second, 'placement common', common)
        if common not in {'1', SOURCE}:
            divided = operate(SOURCE, 'divmod', common)
            assert field(divided, 'remainder') == '0'
            product(SOURCE, common, field(divided, 'result'))
        divided = operate(SOURCE, 'divmod', placement)
        quotient, remainder = field(divided, 'result'), field(divided, 'remainder')
        positioned = field(operate(placement, 'mul', quotient), 'result')
        assert field(operate(positioned, 'add', remainder), 'result') == SOURCE
        # Any common divisor of quotient and remainder divides the odd source,
        # so removing individual powers of two preserves this common component.
        coupled = common_component(quotient, remainder)
        record.write('OUTPUT-REMAINDER COMMON COMPONENT slots=' + str((first, second)) +
                     ' quotient=' + quotient + ' remainder=' + remainder +
                     ' common=' + coupled + ' exact=PASS\n')
        print(SOURCE, 'slots', first, second, 'output/remainder common', coupled)
        if coupled not in {'1', SOURCE}:
            returned = operate(SOURCE, 'divmod', coupled)
            assert field(returned, 'remainder') == '0'
            product(SOURCE, coupled, field(returned, 'result'))
