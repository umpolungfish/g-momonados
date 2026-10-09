"""Collect a source-inscribed square correction through frame returns."""
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
SOURCE = sys.argv[1] if len(sys.argv) > 1 else '8051'

def field(out, label):
    return next(line.split(None, 1)[1] for line in out.splitlines()
                if line.startswith(label + ' '))

def cells(word):
    return [word[1:-3][i:i + 5] for i in range(0, len(word[1:-3]), 5)]

with (ROOT / 'measurements' / ('godel_hex_source_square_collection_' + SOURCE + '.log')).open('a') as record:
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
        width = max(2, len(left), len(right))
        composed = decode(left + [EMPTY] * (2 * width - len(left)) + right)
        return run(['./godel', 'frame-op', composed, str(width), '0', op,
                    str(2 * width), '1'], allow_underflow=(op == 'sub'))

    def product(n, a, b):
        out = run(['./godel', 'product', n, a, b])
        assert 'relation.exact-product     PASS' in out

    def balanced(value, width):
        parts = read(value)
        parts += [EMPTY] * ((-len(parts)) % width)
        frame_values = [decode(parts[i:i + width])
                        for i in range(0, len(parts), width)]
        sums = ['0', '0']
        for i, frame in enumerate(frame_values):
            side = i % 2
            if frame == '0':
                continue
            sums[side] = frame if sums[side] == '0' else field(
                operate(sums[side], 'add', frame), 'result')
        base = decode([EMPTY] * width + [FILLED])
        factor = field(operate(base, 'add', '1'), 'result')
        if sums[0] != sums[1]:
            record.write('RETAINED UNBALANCED FRAMES width=' + str(width) +
                         ' frames=' + repr(frame_values) + ' sums=' + repr(sums) + '\n')
            difference = operate(sums[0], 'sub', sums[1])
            sign = '+'
            if difference is None:
                difference = operate(sums[1], 'sub', sums[0])
                sign = '-'
            magnitude = field(difference, 'result')
            read(magnitude)
            folded = operate(magnitude, 'divmod', factor)
            if field(folded, 'remainder') != '0':
                print(SOURCE, 'retained fold width', width, 'sums', sums,
                      'signed imbalance', sign + magnitude,
                      'factor', factor, 'remainder', field(folded, 'remainder'))
                sys.exit(0)
            record.write('IMBALANCE COLLECTS ' + sign + magnitude + ' = ' +
                         sign + field(folded, 'result') + ' * ' + factor + '\n')
            product(magnitude, factor, field(folded, 'result'))
            print(SOURCE, 'collected fold imbalance', sign + magnitude,
                  'through factor', factor)
        out = operate(value, 'divmod', factor)
        assert field(out, 'remainder') == '0'
        quotient = field(out, 'result')
        product(value, factor, quotient)
        record.write('COMPOSED FOLD RETURN ' + value + ' = ' + factor +
                     ' * ' + quotient + '\n')
        return factor, quotient

    source = SOURCE
    source_parts = read(source)
    payload = decode(source_parts[4:8])
    if payload == '0':
        record.write('RETAINED EMPTY SECOND HEX PAYLOAD\n')
        print(source, 'retained empty second hex payload')
        sys.exit(0)
    square = field(operate(payload, 'mul', payload), 'result')
    correction = field(operate(source, 'add', square), 'result')
    parts = read(correction)
    low_parts = parts[:4]
    low_parts += [EMPTY] * (4 - len(low_parts))
    low = decode(low_parts)
    if low not in {'0', '1', '4', '9'}:
        record.write('SQUARE RETURN REFUTED: corrected low hex value ' + low +
                     ' lies outside the verified square image {0,1,4,9}\n')
        print(source, 'square return refuted for this correction; low hex', low)
        sys.exit(0)
    initial_empty = 0
    for part in parts:
        if part != EMPTY:
            break
        initial_empty += 1
    shift = decode([EMPTY] * initial_empty + [FILLED])
    tail = decode(parts[initial_empty:])
    product(correction, shift, tail)
    first, child = balanced(tail, 2)
    second, grandchild = balanced(child, 2)
    third, fourth = balanced(grandchild, 3)
    if first != second or third != fourth or initial_empty % 2 != 0:
        record.write('RETAINED UNPAIRED RETURNED FACTORS OR ODD INITIAL EMPTY RUN\n')
        print(source, 'retained unpaired returned factors or odd empty run')
        sys.exit(0)
    root_tail = field(operate(first, 'mul', third), 'result')
    product(tail, root_tail, root_tail)
    assert initial_empty % 2 == 0
    root_shift = decode([EMPTY] * (initial_empty // 2) + [FILLED])
    product(shift, root_shift, root_shift)
    root = field(operate(root_shift, 'mul', root_tail), 'result')
    product(correction, root, root)
    left = field(operate(root, 'sub', payload), 'result')
    right = field(operate(root, 'add', payload), 'result')
    product(source, left, right)
    for value in (payload, square, root, left, right):
        read(value)
    record.write('SOURCE-ONLY SQUARE COLLECTION ' + source + ' = ' + left +
                 ' * ' + right + '\n')
    print(source, '+', square, '=', root, '*', root, '; factors', left, right,
          'PASS')
