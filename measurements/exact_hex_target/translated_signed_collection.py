"""Collect one maximal literal two-copy translation of the signed source."""
from pathlib import Path
import re
import subprocess
import gzip
import argparse

parser = argparse.ArgumentParser()
parser.add_argument('--hex-boundaries', action='store_true')
parser.add_argument('--opposite', action='store_true')
parser.add_argument('--displacement', type=int)
parser.add_argument('--higher-first', action='store_true')
options = parser.parse_args()
polarity = -1 if options.opposite else 1
record_name = ('translated_signed_collection' +
               ('_higher_first' if options.higher_first else '') +
               ('_hex' if options.hex_boundaries else '') +
               ('_opposite' if options.opposite else '') +
               ('_' + str(options.displacement) if options.displacement else ''))

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
log = []


def call(*args):
    result = subprocess.run([str(ROOT / 'godel'), *args], cwd=ROOT,
                            capture_output=True, text=True, check=True)
    log.append(repr(args) + '\n' + result.stdout)
    return result.stdout


def field(output, name):
    return next(line.split(None, 1)[1] for line in output.splitlines()
                if re.match(r'^' + re.escape(name) + r'\s', line))


def word(positions):
    positions = set(positions)
    return '⊢' + ''.join(FILLED if i in positions else EMPTY
                         for i in range(max(positions, default=0)+1)) + '⊙⊡⊣'


def operate(left, op, right):
    if right == word([]):
        assert op in ('add', 'sub', 'mul')
        return word([]) if op == 'mul' else left
    lc, rc = left[1:-3], right[1:-3]
    width = max(2, len(lc)//5, len(rc)//5)
    carrier_word = '⊢' + lc + EMPTY*(2*width-len(lc)//5) + rc + '⊙⊡⊣'
    carrier = field(call('decode', carrier_word), 'value')
    value = field(call('frame-op', carrier, str(width), '0', op,
                       str(2*width), '1'), 'result')
    result = field(call('encode', value), 'word')
    check = (call('check', 'add', right, result, left) if op == 'sub'
             else call('check', op, left, right, result))
    assert 'PASS' in check and 'FAIL' not in check
    return result


def signed_magnitude(terms):
    if not terms:
        return '+', word([])
    sign = '+' if terms[max(terms)] > 0 else '-'
    positive = word(i for i,c in terms.items() if c > 0)
    negative = word(i for i,c in terms.items() if c < 0)
    if sign == '+':
        return sign, operate(positive, 'sub', negative)
    return sign, operate(negative, 'sub', positive)


text = (HERE / 'run_payload_collection.md').read_text()
signed = {}
for label, sign in [('Positive', 1), ('Negative', -1)]:
    line = re.search(label + r' unit positions: ([0-9, ]+)', text).group(1)
    signed.update({int(i): sign for i in line.split(', ')})

rewrites = []
if options.higher_first:
    source = (HERE / 'source.txt').read_text().strip()
    native = field(call('encode', source), 'word')
    cells = native[1:-3]
    occupied = {i//5 for i in range(0, len(cells), 5)
                if cells[i:i+5] == FILLED}
    signed = {}
    for i in sorted(occupied):
        if i-1 not in occupied:
            signed[i] = signed.get(i, 0)-1
        if i+1 not in occupied:
            signed[i+1] = signed.get(i+1, 0)+1
    signed = {i:c for i,c in signed.items() if c}
    while True:
        changed = False
        for i in sorted(signed, reverse=True):
            c = signed[i]
            if abs(c) >= 2:
                sign = 1 if c > 0 else -1
                signed[i] -= 2*sign
                signed[i+1] = signed.get(i+1, 0)+sign
                rule = ('merge', i, sign)
            elif signed.get(i+1, 0)*c < 0:
                sign = 1 if signed[i+1] > 0 else -1
                signed[i+1] -= sign
                signed[i] += 2*sign
                rule = ('split-cancel', i, sign)
            else:
                continue
            signed = {j:d for j,d in signed.items() if d}
            rewrites.append((rule, sorted(signed.items())))
            changed = True
            break
        if not changed:
            break
    assert all(abs(c) == 1 for c in signed.values())
    positive = word(i for i,c in signed.items() if c > 0)
    negative = word(i for i,c in signed.items() if c < 0)
    assert operate(native, 'add', negative) == positive

# Choose by repeated source support alone. No divisibility or product-return
# tests participate in selecting the displacement.
best = None
step = 4 if options.hex_boundaries else 1
displacements = range(step, max(signed)+1, step)
if options.displacement:
    assert options.displacement > 0 and options.displacement % step == 0
    displacements = [options.displacement]
for displacement in displacements:
    used, payload = set(), {}
    for i in sorted(signed):
        j = i + displacement
        if i not in used and j not in used and signed.get(j) == polarity*signed[i]:
            payload[i] = signed[i]
            used.update((i, j))
    if best is None or len(payload) > len(best[1]):
        best = displacement, payload, used
displacement, payload, used = best
residual = {i:c for i,c in signed.items() if i not in used}
reconstructed = dict(residual)
for i, c in payload.items():
    for j, sign in ((i, 1), (i+displacement, polarity)):
        reconstructed[j] = reconstructed.get(j, 0) + sign*c
assert reconstructed == signed

positive = word(i for i,c in payload.items() if c > 0)
negative = word(i for i,c in payload.items() if c < 0)
placement_terms = {0:1, displacement:polarity}
bp = word(i for i,c in placement_terms.items() if c > 0)
bn = word(i for i,c in placement_terms.items() if c < 0)
rp = word(i for i,c in residual.items() if c > 0)
rn = word(i for i,c in residual.items() if c < 0)
source = (HERE / 'source.txt').read_text().strip()
native = field(call('encode', source), 'word')
pos_product = operate(operate(positive, 'mul', bp), 'add',
                      operate(negative, 'mul', bn))
neg_product = operate(operate(negative, 'mul', bp), 'add',
                      operate(positive, 'mul', bn))
lhs = operate(operate(native, 'add', neg_product), 'add', rn)
rhs = operate(pos_product, 'add', rp)
assert lhs == rhs
payload_sign, payload_word = signed_magnitude(payload)
residual_sign, residual_word = signed_magnitude(residual)
placement_sign, placement_word = signed_magnitude(placement_terms)
product_sign = '+' if payload_sign == placement_sign else '-'
payload_value = field(call('decode', payload_word), 'value')
residual_value = field(call('decode', residual_word), 'value')
placement_value = field(call('decode', placement_word), 'value')
hex_readings = []
for label, value in [('payload', payload_value), ('cofactor', placement_value),
                     ('residual', residual_value)]:
    command = [str(ROOT / 'run_cmds.sh'), 'tfactor read ' + value]
    result = subprocess.run(command, cwd=ROOT, capture_output=True,
                            text=True, check=True)
    log.append(repr(command) + '\n' + result.stdout)
    clean = re.sub(r'\x1b\[[0-9;]*m', '', result.stdout)
    hex_word = next(line.split(':', 1)[1].strip() for line in clean.splitlines()
                    if 'hex-digit word' in line)
    hex_readings.append((label, hex_word))

# Literal whole-payload copies only. Absorption requires every signed
# payload term at one displacement, with the same or opposite polarity.
copies = []
anchor = min(payload)
for position in sorted(residual):
    shift = position-anchor
    if shift < 0:
        continue
    for sign in (1, -1):
        if all(residual.get(i+shift) == sign*c for i,c in payload.items()):
            copies.append((shift, sign))
assert not copies, 'literal copy requires a further recorded absorption'
report = ['# Translated signed payload collection', '',
          'Selection uses only literal translated source terms. '
          'There is no candidate factor search or divisibility test.', '',
          f'Displacement: {displacement} native cells.',
          f'Translation polarity: {polarity}.',
          f'Whole-hex boundary restriction: {options.hex_boundaries}.',
          f'Higher-first source cancellation: {options.higher_first}.',
          'Source rewrite trace: ' + repr(rewrites), '',
          'Complete signed source positions: ' + repr(sorted(signed.items())), '',
          f'Payload: {len(payload)} signed terms, copied twice.',
          f'Residual: {len(residual)} signed terms.', '',
          'Signed payload positions: ' + repr(sorted(payload.items())), '',
          'Signed residual positions: ' + repr(sorted(residual.items())), '',
          'Exact relation: `N = (A_positive - A_negative) * B + '
          'R_positive - R_negative`, where B has signed unit terms '
          f'{sorted(placement_terms.items())}.', '',
          'Independent Gödel checks verify the unsigned product terms and '
          'the complete equation `N + negative_product + R_negative = '
          'positive_product + R_positive`.', '',
          'The residual is retained. A proper-factor certificate requires '
          'its complete cancellation; none is claimed here.', '',
          f'Complete operand equation: `N = {product_sign}{payload_value} * {placement_value} '
          f'{residual_sign} {residual_value}`.', '',
          'The residual contains no complete translated signed copy of this '
          'payload in either polarity. Literal absorption cannot close this '
          'particular collected return.', '']
for label, hex_word in hex_readings:
    report.extend([f'{label} ordered hex word:', '', hex_word, ''])
(HERE / (record_name + '.md')).write_text('\n'.join(report)+'\n')
with gzip.open(HERE / (record_name + '.log.gz'), 'wt') as out:
    out.write('\n'.join(log))
print(f'displacement={displacement} payload_terms={len(payload)} '
      f'residual_terms={len(residual)} full_source_equation=PASS', flush=True)
