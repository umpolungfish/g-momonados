"""Follow unequal full hex blocks through their retained difference and output remainder."""
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

with (ROOT / 'measurements' / ('godel_hex_unequal_composition_' + SOURCE + '.log')).open('a') as record:
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

    run(['python3', 'measurements/godel_hex_ordered_word_probe.py', SOURCE])
    source_parts = read(SOURCE)
    motif_count = (len(source_parts) + 3) // 4
    if motif_count < 2:
        record.write('SINGLE MOTIF: retain complete word for a different frame choice\n')
        print(SOURCE, 'single motif retained')
        sys.exit(0)
    frame_width = 4 * (motif_count // 2)
    lower = decode(source_parts[:frame_width])
    upper = decode(source_parts[frame_width:])
    multiplier = decode([EMPTY] * frame_width + [FILLED])
    payload = decode([FILLED] + [EMPTY] * (frame_width - 1) + [FILLED])
    read(lower)
    read(upper)
    read(payload)
    delta = operate(lower, 'sub', upper)
    sign = '+'
    if delta is None:
        delta = operate(upper, 'sub', lower)
        sign = '-'
    difference = field(delta, 'result')
    read(difference)
    positioned = field(operate(payload, 'mul', upper), 'result')
    assert field(operate(positioned, 'add' if sign == '+' else 'sub', difference), 'result') == SOURCE
    record.write('UNEQUAL FULL COMPOSITION n=' + SOURCE + ' frame-cells=' + str(frame_width) +
                 ' lower=' + lower + ' upper=' + upper + ' payload=' + payload +
                 ' difference=' + sign + difference + ' EXACT PASS\n')
    returned = operate(SOURCE, 'divmod', upper)
    quotient, remainder = field(returned, 'result'), field(returned, 'remainder')
    if remainder == '0':
        product(SOURCE, upper, quotient)
        proper = upper not in {'0', '1', SOURCE} and quotient not in {'0', '1', SOURCE}
        print(SOURCE, 'upper-frame product', upper, quotient, 'proper-factor PASS' if proper else 'unit return')
        sys.exit(0)
    # The next multiple of the output supplies a correction from its actual remainder.
    square_correction = field(operate(upper, 'sub', remainder), 'result')
    next_quotient = field(operate(quotient, 'add', '1'), 'result')
    corrected = field(operate(upper, 'mul', next_quotient), 'result')
    assert field(operate(SOURCE, 'add', square_correction), 'result') == corrected
    record.write('OUTPUT-REMAINDER CORRECTION upper=' + upper + ' quotient=' + quotient +
                 ' remainder=' + remainder + ' correction-square=' + square_correction +
                 ' target-square=' + corrected + ' EXACT PASS\n')
    correction_root = exact_root(square_correction, 'CORRECTION')
    target_root = exact_root(corrected, 'TARGET')
    if correction_root is None or target_root is None:
        record.write('FOLLOW RETAINED OUTPUT, REMAINDER AND SQUARE PREFIXES; no factor certificate\n')
        print(SOURCE, 'unequal composition retained', 'correction-square=' + square_correction,
              'target-square=' + corrected, 'roots=' + str((correction_root, target_root)))
        sys.exit(0)
    left = field(operate(target_root, 'sub', correction_root), 'result')
    right = field(operate(target_root, 'add', correction_root), 'result')
    product(SOURCE, left, right)
    proper = left not in {'0', '1', SOURCE} and right not in {'0', '1', SOURCE}
    read(left)
    read(right)
    record.write('UNEQUAL COMPOSITION SOURCE PRODUCT left=' + left + ' right=' + right +
                 ' exact=PASS proper=' + str(proper) + '\n')
    print(SOURCE, 'unequal composition factors', left, right, 'proper-factor PASS' if proper else 'unit return')
