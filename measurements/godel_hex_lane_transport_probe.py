"""Verify hex operator exchanges through Gödel's positioned lane braid."""
from godel_hex_record_name import record_name
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
EMPTY = '≻⋈∈⊤∋'
PAIRS = [('truth/falsity', 0, 1), ('truth/information', 0, 2),
         ('truth/fork', 0, 3), ('falsity/information', 1, 2),
         ('falsity/fork', 1, 3), ('information/fork', 2, 3)]

def field(out, label):
    return next(line[len(label):].strip() for line in out.splitlines()
                if line.startswith(label + ' '))

def cells(word):
    return [word[1:-3][i:i + 5] for i in range(0, len(word[1:-3]), 5)]

for source in sys.argv[1:]:
    path = ROOT / 'measurements' / ('godel_hex_lane_transport_' + record_name(source) + '.log')
    with path.open('a') as record:
        def run(args):
            out = subprocess.run(args, cwd=ROOT, text=True,
                                 stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
            record.write('COMMAND ' + repr(args) + '\n' + out.stdout +
                         '\nEXIT ' + str(out.returncode) + '\n\n')
            if out.returncode:
                raise RuntimeError(out.stdout)
            return out.stdout

        def decode(parts):
            word = '⊢' + ''.join(parts) + '⊙⊡⊣'
            return field(run(['./godel', 'decode', word]), 'value')

        def read(value):
            out = run(['./run_cmds.sh', 'trilattice_factor read ' + value])
            return re.sub(r'\x1b\[[0-9;]*m', '', out)

        original = read(source)
        parts = cells(next(line.split(':', 1)[1].strip()
                           for line in original.splitlines()
                           if 'native word      :' in line))
        parts += [EMPTY] * ((-len(parts)) % 4)
        unbraided = run(['./godel', 'unbraid', source])
        lane_width = len(parts) // 2
        lanes = [cells(field(unbraided, 'Γ.word')),
                 cells(field(unbraided, 'Λ.word'))]
        for lane in lanes:
            lane += [EMPTY] * (lane_width - len(lane))
        for label, first, second in PAIRS:
            record.write('POSITIONED OPERATOR EXCHANGE ' + label + '\n')
            direct = parts.copy()
            transported = [lane.copy() for lane in lanes]
            for j in range(len(parts) // 4):
                a, b = 4 * j + first, 4 * j + second
                direct[a], direct[b] = direct[b], direct[a]
                la, lb = first % 2, second % 2
                ia, ib = 2 * j + first // 2, 2 * j + second // 2
                transported[la][ia], transported[lb][ib] = (
                    transported[lb][ib], transported[la][ia])
            expected = decode(direct)
            gamma, lam = [decode(lane) for lane in transported]
            braided = run(['./godel', 'braid', gamma, lam])
            assert field(braided, 'braided.value') == expected
            assert 'ΓΛ-closure                closed' in braided
            read(expected)
            record.write('EXACT LANE CONJUGACY PASS\n\n')
        print(source, 'six positioned hex/lane exchanges PASS', path)
