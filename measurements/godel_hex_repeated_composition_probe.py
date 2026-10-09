"""Extract repeated full hex compositions using their source-selected translation."""
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

with (ROOT / 'measurements' / ('godel_hex_repeated_composition_' + SOURCE + '.log')).open('a') as record:
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

    run(['python3', 'measurements/godel_hex_ordered_word_probe.py', SOURCE])
    out = run(['./run_cmds.sh', 'trilattice_factor read ' + SOURCE])
    clean = re.sub(r'\x1b\[[0-9;]*m', '', out)
    hex_word = next(line.split(':', 1)[1].strip() for line in clean.splitlines() if 'hex-digit word   :' in line)
    motifs = re.findall('⊢[^⊢⊣]*⊣', hex_word)
    native = read(SOURCE)
    # The prefix function selects a word period without numerical factor candidates.
    borders = [0] * len(motifs)
    for position in range(1, len(motifs)):
        border = borders[position - 1]
        while border and motifs[position] != motifs[border]:
            border = borders[border - 1]
        if motifs[position] == motifs[border]:
            border += 1
        borders[position] = border
    period = len(motifs) - borders[-1]
    if len(motifs) % period or period == len(motifs):
        record.write('NONREPETITION SELECTS THE RETAINED UNEQUAL FULL COMPOSITION\n')
        print(SOURCE, 'full hex composition selects its unequal-block return')
        record.flush()
        followed = run(['python3', 'measurements/godel_hex_unequal_composition_probe.py', SOURCE])
        print(followed.rstrip())
        sys.exit(0)
    # Prefer the equal-half fork when available, preserving all internal operators.
    half = len(motifs) // 2
    if len(motifs) % 2 == 0 and motifs[:half] == motifs[half:]:
        period = half
    copies = len(motifs) // period
    frame_width = 4 * period
    repeated_block = native[:frame_width]
    source_parts = native + [EMPTY] * (4 * len(motifs) - len(native))
    assert all(source_parts[start:start + frame_width] == repeated_block
               for start in range(0, len(source_parts), frame_width))
    block = decode(repeated_block)
    payload_parts = [EMPTY] * (frame_width * (copies - 1)) + [FILLED]
    for start in range(0, len(payload_parts), frame_width):
        payload_parts[start] = FILLED
    payload = decode(payload_parts)
    product(SOURCE, block, payload)
    read(block)
    read(payload)
    proper = block not in {'0', '1', SOURCE} and payload not in {'0', '1', SOURCE}
    record.write('REPEATED COMPOSITION RETURN motif-width=' + str(period) +
                 ' copies=' + str(copies) + ' block=' + block + ' payload=' + payload +
                 ' exact=PASS proper=' + str(proper) + '\n')
    print(SOURCE, 'repeated hex composition factors', block, payload,
          'proper-factor PASS' if proper else 'unit return')
