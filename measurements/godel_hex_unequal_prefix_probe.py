"""Follow a failed unequal composition through its source-selected square residue and prefix slot."""
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

with (ROOT / 'measurements' / ('godel_hex_unequal_prefix_' + SOURCE + '.log')).open('a') as record:
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
    width = 4 * (((len(source_parts) + 3) // 4) // 2)
    if width == 0:
        print(SOURCE, 'single motif retained')
        sys.exit(0)
    upper = decode(source_parts[width:])
    upper_parts = read(upper)
    if source_parts[0] != FILLED or upper_parts[0] != FILLED:
        record.write('RETAIN EVEN SOURCE OR EVEN UPPER BLOCK FOR ITS VALUATION FRAME\n')
        print(SOURCE, 'even source or upper block retained')
        sys.exit(0)
    low_source = decode(source_parts[:4])
    desired = {'1':'1', '3':'4', '5':'9', '7':'0', '9':'9', '11':'4', '13':'1', '15':'0'}[low_source]
    # Lift the inverse of the odd upper motif one positioned unit at a time.
    inverse_parts = [FILLED]
    for position in range(1, 4):
        inverse = decode(inverse_parts)
        inverted = field(operate(upper, 'mul', inverse), 'result')
        parts = encode(inverted)
        parts += [EMPTY] * max(0, position + 1 - len(parts))
        inverse_parts += [EMPTY] * (position + 1 - len(inverse_parts))
        if parts[position] == FILLED:
            inverse_parts[position] = FILLED
        inverse = decode(inverse_parts)
        inverted = field(operate(upper, 'mul', inverse), 'result')
        parts = encode(inverted)
        parts += [EMPTY] * max(0, position + 1 - len(parts))
        assert parts[:position + 1] == [FILLED] + [EMPTY] * position
        record.write('ODD UPPER MOTIF INVERSE position=' + str(position) + ' inverse=' + inverse + ' PASS\n')
    if desired == '0':
        product('0', inverse, '0')
        selected_low = '0'
    else:
        selected_product = field(operate(desired, 'mul', inverse), 'result')
        selected_low = decode(encode(selected_product)[:4])
    returned = operate(SOURCE, 'divmod', upper)
    original = field(operate(field(returned, 'result'), 'add', '1'), 'result')
    read(original)
    coefficient_parts = encode(original)
    coefficient_parts += [EMPTY] * max(0, 4 - len(coefficient_parts))
    low_parts = encode(selected_low)
    low_parts += [EMPTY] * (4 - len(low_parts))
    coefficient_parts[:4] = low_parts
    coefficient = decode(coefficient_parts)
    if operate(coefficient, 'sub', original) is None:
        coefficient = field(operate(coefficient, 'add', decode([EMPTY] * 4 + [FILLED])), 'result')
    read(coefficient)
    target = field(operate(coefficient, 'mul', upper), 'result')
    correction = field(operate(target, 'sub', SOURCE), 'result')
    assert decode(encode(target)[:4]) == desired
    assert decode(encode(correction)[:4]) in {'0', '1', '4', '9'}
    record.write('SOURCE HEX SELECTED COEFFICIENT source-low=' + low_source + ' target-low=' + desired +
                 ' coefficient-low=' + selected_low + ' coefficient=' + coefficient + '\n')
    def prefix_slot(value):
        if value == '0':
            return None
        parts = encode(value)
        zeros = 0
        for cell in parts:
            if cell != EMPTY:
                break
            zeros += 1
        if zeros % 2:
            return zeros
        tail = parts[zeros:]
        tail += [EMPTY] * max(0, 3 - len(tail))
        for index in (1, 2):
            if tail[index] == FILLED:
                return zeros + index
        return None
    failures = [(slot, label) for label, value in (('TARGET', target), ('CORRECTION', correction))
                if (slot := prefix_slot(value)) is not None]
    if failures:
        slot, label = min(failures)
        assert slot >= 4
        unit = decode([EMPTY] * slot + [FILLED])
        increment = field(operate(upper, 'mul', unit), 'result')
        coefficient = field(operate(coefficient, 'add', unit), 'result')
        next_target = field(operate(coefficient, 'mul', upper), 'result')
        assert field(operate(target, 'add', increment), 'result') == next_target
        correction = field(operate(correction, 'add', increment), 'result')
        target = next_target
        slots = ('truth ⊤', 'falsity ⊥', 'information ⊞', 'fork ∈/⋈/∋')
        record.write('ONE PREFIX SELECTED UNIT label=' + label + ' source-cell=' + str(slot) +
                     ' operator-slot=' + slots[slot % 4] + ' coefficient=' + coefficient + '\n')
    assert field(operate(SOURCE, 'add', correction), 'result') == target
    record.write('REPAIRED UNEQUAL RETURN n=' + SOURCE + ' correction-square=' + correction +
                 ' target-square=' + target + ' EXACT PASS\n')
    read(coefficient)
    a, h = exact_root(correction, 'CORRECTION'), exact_root(target, 'TARGET')
    if a is None or h is None:
        record.write('RETAIN REPAIRED WORDS AND THEIR NEW PREFIX PATTERN\n')
        print(SOURCE, 'source-selected unequal repair', 'coefficient=' + coefficient,
              'roots=' + str((a, h)))
        sys.exit(0)
    left = field(operate(h, 'sub', a), 'result')
    right = field(operate(h, 'add', a), 'result')
    product(SOURCE, left, right)
    proper = left not in {'0', '1', SOURCE} and right not in {'0', '1', SOURCE}
    read(left)
    read(right)
    record.write('REPAIRED SOURCE PRODUCT left=' + left + ' right=' + right + ' exact=PASS proper=' + str(proper) + '\n')
    print(SOURCE, 'repaired unequal factors', left, right, 'proper-factor PASS' if proper else 'unit return')
