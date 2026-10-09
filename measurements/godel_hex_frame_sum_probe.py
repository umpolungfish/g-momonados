"""Test complete-source frame sums and their retained low-prefix constraints."""
from godel_hex_record_name import record_name
import gzip
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

SOURCE = sys.argv[1]
REPAIR_LOW_SUM = '--repair-low-sum' in sys.argv[2:]

with gzip.open(ROOT / 'measurements' / ('godel_hex_frame_sum_' + record_name(SOURCE) + ('_low_sum' if REPAIR_LOW_SUM else '') + '.log.gz'), 'at') as record:
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
        out = run(['./run_cmds.sh', 'tfactor read ' + value])
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

    def exact_root(value, label):
        read(value)
        if value == '0':
            product('0', '0', '0')
            return '0'
        target = encode(value)
        zeros = 0
        for cell in target:
            if cell != EMPTY:
                break
            zeros += 1
        if zeros % 2:
            record.write(label + ' SQUARE PREFIX: odd empty run ' + str(zeros) + '\n')
            return None
        tail = decode(target[zeros:])
        shift = decode([EMPTY] * zeros + [FILLED])
        product(value, shift, tail)
        target = encode(tail)
        target += [EMPTY] * max(0, 3 - len(target))
        if target[:3] != [FILLED, EMPTY, EMPTY]:
            record.write(label + ' SQUARE PREFIX: odd tail differs from 1 modulo 8\n')
            slots = ('truth ⊤', 'falsity ⊥', 'information ⊞', 'fork ∈/⋈/∋')
            expected = [FILLED, EMPTY, EMPTY]
            for index, cell in enumerate(target[:3]):
                if cell != expected[index]:
                    record.write(label + ' FIRST-PREFIX MISMATCH source-cell=' + str(zeros + index) +
                                 ' operator-slot=' + slots[(zeros + index) % 4] + '\n')
            return None
        root_parts = [FILLED]
        bound = (len(encode(tail)) + 1) // 2 + 1
        for position in range(3, max(4, bound + 1)):
            current = decode(root_parts)
            square = field(operate(current, 'mul', current), 'result')
            square_parts = encode(square)
            square_parts += [EMPTY] * max(0, position + 1 - len(square_parts))
            target += [EMPTY] * max(0, position + 1 - len(target))
            if square_parts[position] != target[position]:
                root_parts += [EMPTY] * (position - len(root_parts))
                assert root_parts[position - 1] == EMPTY
                root_parts[position - 1] = FILLED
                current = decode(root_parts)
                square = field(operate(current, 'mul', current), 'result')
                square_parts = encode(square)
                square_parts += [EMPTY] * max(0, position + 1 - len(square_parts))
            assert square_parts[:position + 1] == target[:position + 1]
            modulus = decode([EMPTY] * position + [FILLED])
            opposite = field(operate(modulus, 'sub', current), 'result')
            opposite_square = field(operate(opposite, 'mul', opposite), 'result')
            record.write(label + ' BIT LIFT position=' + str(position) + ' root=' + current +
                         ' opposite=' + opposite + ' prefix=closed\n')
            if square == tail or opposite_square == tail:
                selected = current if square == tail else opposite
                root_shift = decode([EMPTY] * (zeros // 2) + [FILLED])
                root = field(operate(root_shift, 'mul', selected), 'result')
                product(value, root, root)
                read(root)
                return root
        record.write(label + ' RETAINED ROOT WORDS ' + current + ' ' + opposite + '\n')
        read(current)
        read(opposite)
        return None

    source_parts = read(SOURCE)
    assert len(source_parts) >= 100, "Source must have at least 100 native cells"
    motif_count = (len(source_parts) + 3) // 4
    frame_width = 4 * (motif_count // 2)
    while frame_width >= 100:
        upper = decode(source_parts[frame_width:])
        boundary = decode([EMPTY] * frame_width + [FILLED])
        selected_sum = field(operate(boundary, 'add', upper), 'result')
        if REPAIR_LOW_SUM:
            assert decode(source_parts[:3]) == '7', 'This sum constraint is for source residue 7 modulo 8'
            retained = encode(selected_sum)
            retained[:3] = [EMPTY] * 3
            repaired_sum = decode(retained)
            delta = field(operate(selected_sum, 'sub', repaired_sum), 'result')
            record.write('RETAIN FULL SOURCE SUM; CLEAR LOW THREE CELLS delta=' + delta + ' old-sum=' + selected_sum + ' repaired-sum=' + repaired_sum + '\n')
            selected_sum = repaired_sum
        sum_square = field(operate(selected_sum, 'mul', selected_sum), 'result')
        four_source = field(operate(SOURCE, 'mul', '4'), 'result')
        discriminant = operate(sum_square, 'sub', four_source)
        assert discriminant is not None
        difference_square = field(discriminant, 'result')
        record.write('SOURCE FRAME SUM width=' + str(frame_width) + ' sum=' + selected_sum + ' discriminant=' + difference_square + '\n')
        difference = exact_root(difference_square, 'FRAME SUM')
        if difference is not None:
            left_twice = field(operate(selected_sum, 'sub', difference), 'result')
            right_twice = field(operate(selected_sum, 'add', difference), 'result')
            left_return = operate(left_twice, 'divmod', '2')
            right_return = operate(right_twice, 'divmod', '2')
            assert field(left_return, 'remainder') == field(right_return, 'remainder') == '0'
            left, right = field(left_return, 'result'), field(right_return, 'result')
            product(SOURCE, left, right)
            words = ['⊢' + ''.join(encode(value)) + '⊙⊡⊣' for value in (left, right, SOURCE)]
            native_proof = run(['./godel', 'check', 'mul', *words])
            assert 'PASS' in native_proof and 'FAIL' not in native_proof
            proper = left not in {'0','1',SOURCE} and right not in {'0','1',SOURCE}
            print('FRAME SUM FACTORS', frame_width, left, right, 'proper', proper, flush=True)
            if proper:
                break
        else:
            print('frame sum', frame_width, 'does not give an exact source product', flush=True)
        frame_width = 4 * ((frame_width // 4) // 2)
