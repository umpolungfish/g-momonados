"""Collect translated source lanes with their original positions."""
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

with (ROOT / 'measurements' / ('godel_hex_translated_lane_' + SOURCE + '.log')).open('a') as record:
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
    if gamma[0] != FILLED or FILLED not in lam:
        record.write('RETAINED: odd-source translated-lane condition not applicable\n')
        print(SOURCE, 'retained empty lane or even source')
        sys.exit(0)
    translation = 0
    for cell in lam:
        if cell != EMPTY:
            break
        translation += 1
    a = decode([part for cell in gamma for part in (cell, EMPTY)])
    b = decode([part for cell in lam[translation:] for part in (cell, EMPTY)])
    multiplier = decode([EMPTY] * (2 * translation + 1) + [FILLED])
    payload = decode([FILLED] + [EMPTY] * (2 * translation) + [FILLED])
    read(payload)
    difference = operate(b, 'sub', a)
    sign = '+'
    if difference is None:
        difference = operate(a, 'sub', b)
        sign = '-'
    magnitude = field(difference, 'result')
    if magnitude == '0':
        product('0', multiplier, '0')
        positioned = '0'
    else:
        positioned = field(operate(multiplier, 'mul', magnitude), 'result')
    main_product = field(operate(payload, 'mul', a), 'result')
    recovered = main_product if positioned == '0' else field(
        operate(main_product, 'add' if sign == '+' else 'sub', positioned), 'result')
    assert recovered == SOURCE
    record.write('TRANSLATED LANE RETURN shift=' + str(translation) + ' n=' + SOURCE +
                 ' payload=' + payload + ' placement=' + a + ' residual=' +
                 sign + positioned + '\n')
    read(positioned)
    if magnitude == '0':
        cofactor = a
    else:
        collected = operate(magnitude, 'divmod', payload)
        record.write('TRANSLATED DIFFERENCE COLLECTION quotient=' + field(collected, 'result') +
                     ' remainder=' + field(collected, 'remainder') + '\n')
        if field(collected, 'remainder') != '0':
            print(SOURCE, 'translation', translation, 'payload', payload,
                  'retained difference remainder', field(collected, 'remainder'))
            sys.exit(0)
        adjustment = field(operate(multiplier, 'mul', field(collected, 'result')), 'result')
        cofactor = field(operate(a, 'add' if sign == '+' else 'sub', adjustment), 'result')
    product(SOURCE, payload, cofactor)
    proper = payload != SOURCE and cofactor not in {'0', '1', SOURCE}
    record.write('SOURCE CERTIFICATE exact=PASS proper=' + str(proper) + '\n')
    read(cofactor)
    print(SOURCE, 'translated lane collection', payload, cofactor,
          'proper-factor PASS' if proper else 'unit return')

