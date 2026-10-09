"""Collect literal filled runs. No candidate operands or numeric factoring."""
from pathlib import Path
from collections import defaultdict
import subprocess

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
logs = []


def call(*args):
    result = subprocess.run([str(ROOT / 'godel'), *args], cwd=ROOT,
                            text=True, capture_output=True, check=True)
    logs.append('COMMAND ' + repr(args) + '\n' + result.stdout)
    return result.stdout


def word(positions):
    positions = set(positions)
    return '⊢' + ''.join(FILLED if i in positions else EMPTY
                         for i in range(max(positions, default=0) + 1)) + '⊙⊡⊣'


source = (HERE / 'source.txt').read_text().strip()
encoded = call('encode', source)
native = next(line.split(None, 1)[1] for line in encoded.splitlines()
              if line.startswith('word '))
body = native[1:-3]
cells = [body[i:i+5] for i in range(0, len(body), 5)]
assert all(c in (EMPTY, FILLED) for c in cells)
groups = defaultdict(list)
i = 0
while i < len(cells):
    if cells[i] == EMPTY:
        i += 1
        continue
    start = i
    while i < len(cells) and cells[i] == FILLED:
        i += 1
    groups[i-start].append(start)

lines = ['# Literal source run collection', '',
         'Every maximal filled run is collected by its exact length. '
         'The payload is the literal filled word of that length; the placement '
         'word contains its observed start positions. No unknown factor cells '
         'are introduced.', '',
         '| Payload cells | Observed start positions | Product check |',
         '| --- | --- | --- |']
collected = set()
for length, starts in sorted(groups.items()):
    positions = {start+j for start in starts for j in range(length)}
    assert not collected.intersection(positions)
    product = word(positions)
    result = call('check', 'mul', word(range(length)), word(starts), product)
    assert 'PASS' in result and 'FAIL' not in result
    new_collected = collected | positions
    result = call('check', 'add', word(collected), product, word(new_collected))
    assert 'PASS' in result and 'FAIL' not in result
    collected = new_collected
    lines.append(f'| {length} | ' + ', '.join(map(str, starts)) + ' | PASS |')
assert word(collected) == native
lines += ['', f'{sum(map(len, groups.values()))} observed runs form '
          f'{len(groups)} payload groups. Every group product and successive '
          'addition passes the native Gödel instrument. Their complete sum '
          'reconstructs the source word exactly.', '',
          'No single proper payload covers the complete source. This is an '
          'exact distributive collection, not a factor certificate. The '
          'remaining groups stay explicit; none is discarded.']

# A filled run [s,s+l) is U_(s+l)-U_s. Cancel adjacent opposite
# boundaries using U_(i+1)=2 U_i. Only observed signed terms participate.
signed = {}
for length, starts in groups.items():
    for start in starts:
        signed[start+length] = signed.get(start+length, 0) + 1
        signed[start] = signed.get(start, 0) - 1
signed = {i:c for i,c in signed.items() if c}
initial_terms = len(signed)
moves = []
seen = set()
while True:
    state = tuple(sorted(signed.items()))
    assert state not in seen, 'signed cancellation cycle'
    seen.add(state)
    for i in sorted(signed):
        c = signed[i]
        if abs(c) >= 2:
            sign = 1 if c > 0 else -1
            signed[i] -= 2*sign
            signed[i+1] = signed.get(i+1, 0) + sign
            moves.append(('merge', i, sign))
        elif c*signed.get(i+1, 0) < 0:
            sign = signed[i+1]
            signed[i+1] -= sign
            signed[i] += 2*sign
            moves.append(('split-cancel', i, sign))
        else:
            continue
        signed = {j:d for j,d in signed.items() if d}
        break
    else:
        break
assert all(abs(c) == 1 for c in signed.values())
positive = [i for i,c in signed.items() if c > 0]
negative = [i for i,c in signed.items() if c < 0]
result = call('check', 'add', native, word(negative), word(positive))
assert 'PASS' in result and 'FAIL' not in result
lines += ['', '## Signed boundary cancellation', '',
          f'{initial_terms} run boundaries become {len(signed)} signed terms '
          f'after {len(moves)} local cancellations.', '',
          'Gödel verifies `source + negative-word = positive-word`.', '',
          'Positive unit positions: ' + ', '.join(map(str, sorted(positive))), '',
          'Negative unit positions: ' + ', '.join(map(str, sorted(negative))), '',
          'Local rewrite trace: ' + repr(moves), '',
          'This equation preserves the complete source; it does not yet '
          'supply a closed two-operand product.']
(HERE / 'run_payload_collection.md').write_text('\n'.join(lines) + '\n')
import gzip
with gzip.open(HERE / 'run_payload_collection.log.gz', 'wt') as out:
    out.write('\n'.join(logs))
print(f'Literal groups: {len(groups)}. Signed terms: {initial_terms} -> '
      f'{len(signed)}. Local rewrites: {len(moves)}. Whole-source check: PASS.')
