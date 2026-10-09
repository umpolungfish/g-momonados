"""Certify a source-derived square correction by positioned one-bit lifts."""
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

with (ROOT / 'measurements' / ('godel_hex_square_bit_lift_' + SOURCE + '.log')).open('a') as record:
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

    source = SOURCE
    source_parts = read(source)
    correction_parts = source_parts[4:8]
    correction_parts += [EMPTY] * (4 - len(correction_parts))
    low_source = decode(source_parts[:4])
    if low_source in {'1', '9'}:
        # Move the source's sole falsity presence into information.
        if correction_parts[1] == FILLED and correction_parts[2] == EMPTY:
            correction_parts[1], correction_parts[2] = correction_parts[2], correction_parts[1]
    elif low_source in {'7', '11'}:
        # Retain information and its banked fork; remove falsity when equal.
        if correction_parts[1] == correction_parts[2] == FILLED:
            correction_parts[1] = EMPTY
    payload = decode(correction_parts)
    read(payload)
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
    target = encode(tail)
    target += [EMPTY] * max(0, 3 - len(target))
    if initial_empty % 2 or target[:3] != [FILLED, EMPTY, EMPTY]:
        record.write('SQUARE RETURN REFUTED BY EMPTY-RUN PARITY OR ODD LOW THREE CELLS\n')
        print(source, 'correction', payload, 'retained prefix refutation')
        sys.exit(0)
    root_parts = [FILLED]
    root_tail = None
    bound = (len(encode(tail)) + 1) // 2 + 1
    for position in range(3, max(4, bound + 1)):
        current = decode(root_parts)
        current_square = field(operate(current, 'mul', current), 'result')
        square_parts = encode(current_square)
        square_parts += [EMPTY] * max(0, position + 1 - len(square_parts))
        target += [EMPTY] * max(0, position + 1 - len(target))
        if square_parts[position] != target[position]:
            root_parts += [EMPTY] * (position - len(root_parts))
            assert root_parts[position - 1] == EMPTY
            root_parts[position - 1] = FILLED
            current = decode(root_parts)
            current_square = field(operate(current, 'mul', current), 'result')
            square_parts = encode(current_square)
            square_parts += [EMPTY] * max(0, position + 1 - len(square_parts))
        assert square_parts[:position + 1] == target[:position + 1]
        modulus = decode([EMPTY] * position + [FILLED])
        opposite = field(operate(modulus, 'sub', current), 'result')
        opposite_square = field(operate(opposite, 'mul', opposite), 'result')
        record.write('BIT LIFT position=' + str(position) + ' root=' + current +
                     ' opposite=' + opposite + ' prefix=closed\n')
        if current_square == tail:
            root_tail = current
            break
        if opposite_square == tail:
            root_tail = opposite
            break
    if root_tail is None:
        for value in (current, opposite, current_square, opposite_square):
            read(value)
        record.write('FINITE ROOT WIDTH REACHED WITHOUT EXACT SQUARE RETURN\n')
        print(source, 'correction', payload, 'bit-lift prefixes closed through',
              position, 'retained final roots', current, opposite)
        sys.exit(0)
    product(tail, root_tail, root_tail)
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
