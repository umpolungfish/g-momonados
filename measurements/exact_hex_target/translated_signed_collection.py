"""Collect one maximal literal two-copy translation of the signed source."""
from pathlib import Path
import re
import subprocess
import gzip

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
        assert op == 'add'
        return left
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

# Choose by repeated source support alone. No divisibility or product-return
# tests participate in selecting the displacement.
best = None
for displacement in range(1, max(signed)+1):
    used, payload = set(), {}
    for i in sorted(signed):
        j = i + displacement
        if i not in used and j not in used and signed.get(j) == signed[i]:
            payload[i] = signed[i]
            used.update((i, j))
    if best is None or len(payload) > len(best[1]):
        best = displacement, payload, used
displacement, payload, used = best
residual = {i:c for i,c in signed.items() if i not in used}
reconstructed = dict(residual)
for i, c in payload.items():
    for j in (i, i+displacement):
        reconstructed[j] = reconstructed.get(j, 0) + c
assert reconstructed == signed

positive = word(i for i,c in payload.items() if c > 0)
negative = word(i for i,c in payload.items() if c < 0)
placement = word([0, displacement])
rp = word(i for i,c in residual.items() if c > 0)
rn = word(i for i,c in residual.items() if c < 0)
source = (HERE / 'source.txt').read_text().strip()
native = field(call('encode', source), 'word')
pos_product = operate(positive, 'mul', placement)
neg_product = operate(negative, 'mul', placement)
lhs = operate(operate(native, 'add', neg_product), 'add', rn)
rhs = operate(pos_product, 'add', rp)
assert lhs == rhs
payload_sign, payload_word = signed_magnitude(payload)
residual_sign, residual_word = signed_magnitude(residual)
payload_value = field(call('decode', payload_word), 'value')
residual_value = field(call('decode', residual_word), 'value')
hex_readings = []
for label, value in [('payload', payload_value), ('residual', residual_value)]:
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
          'Selection uses only literal equal-polarity translated source terms. '
          'There is no candidate factor search or divisibility test.', '',
          f'Displacement: {displacement} native cells.',
          f'Payload: {len(payload)} signed terms, copied twice.',
          f'Residual: {len(residual)} signed terms.', '',
          'Signed payload positions: ' + repr(sorted(payload.items())), '',
          'Signed residual positions: ' + repr(sorted(residual.items())), '',
          'Exact relation: `N = (A_positive - A_negative) * B + '
          'R_positive - R_negative`, where B has occupied cells 0 and '
          f'{displacement}.', '',
          'Independent Gödel checks verify both unsigned products and the '
          'complete equation `N + A_negative*B + R_negative = '
          'A_positive*B + R_positive`.', '',
          'The residual is retained. A proper-factor certificate requires '
          'its complete cancellation; none is claimed here.', '',
          f'Complete operand equation: `N = {payload_sign}{payload_value} * 5 '
          f'{residual_sign} {residual_value}`.', '',
          'The residual contains no complete translated signed copy of this '
          'payload in either polarity. Literal absorption cannot close this '
          'particular collected return.', '']
for label, hex_word in hex_readings:
    report.extend([f'{label} ordered hex word:', '', hex_word, ''])
(HERE / 'translated_signed_collection.md').write_text('\n'.join(report)+'\n')
with gzip.open(HERE / 'translated_signed_collection.log.gz', 'wt') as out:
    out.write('\n'.join(log))
print(f'displacement={displacement} payload_terms={len(payload)} '
      f'residual_terms={len(residual)} full_source_equation=PASS', flush=True)
