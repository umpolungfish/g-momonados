"""Retain the product defect of a source-derived hex operator exchange."""
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
SOURCE = sys.argv[1] if len(sys.argv) > 1 else '99'

def field(out, label):
    return next(line.split(None, 1)[1] for line in out.splitlines()
                if line.startswith(label + ' '))

def spell(parts):
    return '⊢' + ''.join(parts) + '⊙⊡⊣'

def cells(word):
    return [word[1:-3][i:i + 5] for i in range(0, len(word[1:-3]), 5)]

with (ROOT / 'measurements' / ('godel_hex_product_transport_' + SOURCE + '.log')).open('a') as record:
    def run(args):
        result = subprocess.run(args, cwd=ROOT, text=True,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        record.write('COMMAND ' + repr(args) + '\n' + result.stdout +
                     '\nEXIT ' + str(result.returncode) + '\n\n')
        if result.returncode:
            raise RuntimeError(result.stdout)
        return result.stdout

    def decode(parts):
        return field(run(['./godel', 'decode', spell(parts)]), 'value')

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
                    str(2 * width), '1'])

    def exchange(value):
        parts = read(value)
        parts += [EMPTY] * ((-len(parts)) % 4)
        for i in range(0, len(parts), 4):
            parts[i], parts[i + 2] = parts[i + 2], parts[i]
        result = decode(parts)
        read(result)
        return result

    def product(n, a, b):
        out = run(['./godel', 'product', n, a, b])
        assert 'relation.exact-product     PASS' in out

    source = SOURCE
    transformed = exchange(source)
    parts = read(transformed)
    initial_empty = 0
    for part in parts:
        if part != EMPTY:
            break
        initial_empty += 1
    if not initial_empty:
        record.write('RETAINED: transformed source has no initial empty cells\n')
        print(source, 'transformed', transformed, 'retained odd source')
        sys.exit(0)
    shift = decode([EMPTY] * initial_empty + [FILLED])
    tail = decode(parts[initial_empty:])
    product(transformed, shift, tail)
    # Equal width-three frames of the returned tail select the payload and
    # placement, without supplying a factor numeral.
    tail_parts = read(tail)
    tail_parts += [EMPTY] * ((-len(tail_parts)) % 3)
    frames = [tail_parts[i:i + 3] for i in range(0, len(tail_parts), 3)]
    if len(frames) < 2 or any(frame != frames[0] for frame in frames[1:]):
        record.write('RETAINED UNEQUAL WIDTH-THREE FRAMES ' + repr(frames) + '\n')
        print(source, 'transformed', transformed, 'shift', shift, 'tail', tail,
              'retained unequal frames')
        sys.exit(0)
    payload = decode(frames[0])
    placement = decode([FILLED, EMPTY, EMPTY] * len(frames))
    product(tail, payload, placement)
    left = field(operate(shift, 'mul', payload), 'result')
    right = placement
    product(transformed, left, right)
    inverse_left, inverse_right = exchange(left), exchange(right)
    inverse_product = field(operate(inverse_left, 'mul', inverse_right), 'result')
    residual = field(operate(source, 'sub', inverse_product), 'result')
    read(residual)
    collection = operate(residual, 'divmod', inverse_left)
    assert field(collection, 'remainder') == '0'
    cofactor = field(operate(inverse_right, 'add', field(collection, 'result')),
                     'result')
    product(source, inverse_left, cofactor)
    record.write('SOURCE-DERIVED INVERSE TRANSPORT COLLECTION ' +
                 inverse_left + ' * ' + cofactor + ' = ' + source + '\n')
    print(source, 'exchange', transformed, 'child product', left, right,
          'inverse product', inverse_product, 'residual', residual,
          'collected factors', inverse_left, cofactor, 'PASS')
