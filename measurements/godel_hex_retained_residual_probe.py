"""Return the retained 8051 correction through existing frame operations."""
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'

def field(out, label):
    return next(line.split(None, 1)[1] for line in out.splitlines()
                if line.startswith(label + ' '))

def cells(word):
    return [word[1:-3][i:i + 5] for i in range(0, len(word[1:-3]), 5)]

with (ROOT / 'measurements/godel_hex_retained_residual_8051.log').open('a') as record:
    def run(args):
        out = subprocess.run(args, cwd=ROOT, text=True,
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        record.write('COMMAND ' + repr(args) + '\n' + out.stdout +
                     '\nEXIT ' + str(out.returncode) + '\n\n')
        if out.returncode:
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
                    str(2 * width), '1'])

    def product(n, a, b):
        out = run(['./godel', 'product', n, a, b])
        assert 'relation.exact-product     PASS' in out

    source = '8051'
    source_parts = read(source)
    payload = decode(source_parts[:2])
    square = field(operate(payload, 'mul', payload), 'result')
    correction = field(operate(source, 'add', square), 'result')
    parts = read(correction)
    initial_empty = 0
    for part in parts:
        if part != EMPTY:
            break
        initial_empty += 1
    shift = decode([EMPTY] * initial_empty + [FILLED])
    tail = decode(parts[initial_empty:])
    product(correction, shift, tail)
    tail_parts = read(tail)
    tail_parts += [EMPTY] * ((-len(tail_parts)) % 2)
    frame_values = [decode(tail_parts[i:i + 2])
                    for i in range(0, len(tail_parts), 2)]
    even_sum, odd_sum = '0', '0'
    # Add the first frame directly so the adapter needs no high zero group.
    for i, value in enumerate(frame_values):
        if i % 2 == 0:
            even_sum = value if even_sum == '0' else field(operate(even_sum, 'add', value), 'result')
        else:
            odd_sum = value if odd_sum == '0' else field(operate(odd_sum, 'add', value), 'result')
    assert even_sum == odd_sum
    base = decode([EMPTY, EMPTY, FILLED])
    balance_factor = field(operate(base, 'add', '1'), 'result')
    balanced = operate(tail, 'divmod', balance_factor)
    assert field(balanced, 'remainder') == '0'
    child = field(balanced, 'result')
    product(tail, balance_factor, child)
    child_parts = read(child)
    width = (len(child_parts) + 1) // 2
    low = decode(child_parts[:width])
    high = decode(child_parts[width:])
    filled = decode([FILLED] * width)
    assert field(operate(low, 'add', high), 'result') == filled
    complement_factor = field(operate(high, 'add', '1'), 'result')
    product(child, filled, complement_factor)
    outer = field(operate(shift, 'mul', balance_factor), 'result')
    product(correction, outer, child)
    for value in (payload, square, outer, filled, complement_factor):
        read(value)
    record.write('RETAINED SOURCE RETURN ' + source + ' + ' + square + ' = ' +
                 outer + ' * ' + filled + ' * ' + complement_factor + '\n')
    print(source, '+', square, '=', outer, '*', filled, '*', complement_factor,
          'source correction returned; source factor target remains open')
