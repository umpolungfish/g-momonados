"""Collect a common lane payload across translated output and residual."""
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

SOURCE = sys.argv[1] if len(sys.argv) > 1 else '117'

with (ROOT / 'measurements' / ('godel_hex_translated_shared_return_' + SOURCE + '.log')).open('a') as record:
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

    run(['python3', 'measurements/godel_hex_translated_lane_probe.py', SOURCE])
    translated_log = (ROOT / 'measurements' / ('godel_hex_translated_lane_' + SOURCE + '.log')).read_text()
    returns = re.findall(r'PARTIAL TRANSLATED RETURN n=(\d+) payload=(\d+) cofactor=(\d+) residual=([+-])(\d+) EXACT PASS', translated_log)
    if not returns or returns[-1][0] != SOURCE:
        raise RuntimeError('No partial translated return for this source')
    _, payload, placement, sign, residual = returns[-1]
    out = run(['python3', 'measurements/godel_hex_lane_residual_probe.py', payload])
    match = re.search(r'lane residual collection (\d+) (\d+) proper-factor PASS', out)
    if match is None:
        raise RuntimeError('Payload lane return did not expose a proper product')
    left, right = match.groups()
    residual_parts = read(residual)
    shift = 0
    for cell in residual_parts:
        if cell != EMPTY:
            break
        shift += 1
    odd_residual = decode(residual_parts[shift:])
    scale = decode([EMPTY] * shift + [FILLED])
    if odd_residual in (left, right):
        factor = odd_residual
        payload_cofactor = right if factor == left else left
        residual_cofactor = scale
        product(residual, factor, scale)
        record.write('RESIDUAL WORD MATCH shift=' + str(shift) + ' odd-word=' + factor + '\n')
    else:
        out = run(['python3', 'measurements/godel_hex_lane_residual_probe.py', residual])
        match = re.search(r'lane residual collection (\d+) (\d+) proper-factor PASS', out)
        if match is None:
            raise RuntimeError('Residual structural return does not match the payload product')
        factor, residual_cofactor = match.groups()
        assert factor in (left, right)
        payload_cofactor = right if factor == left else left
    product(payload, factor, payload_cofactor)
    product(residual, factor, residual_cofactor)
    positioned = field(operate(payload_cofactor, 'mul', placement), 'result')
    cofactor = field(operate(positioned, 'add' if sign == '+' else 'sub', residual_cofactor), 'result')
    product(SOURCE, factor, cofactor)
    assert factor not in {'0', '1', SOURCE} and cofactor not in {'0', '1', SOURCE}
    read(factor)
    read(cofactor)
    record.write('SHARED OUTPUT-RESIDUAL COLLECTION source=' + SOURCE + ' factor=' + factor + ' cofactor=' + cofactor + ' exact=PASS proper=PASS\n')
    print(SOURCE, 'shared output-residual factors', factor, cofactor, 'PASS')
