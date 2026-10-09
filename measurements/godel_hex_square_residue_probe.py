"""Read the complete hex alphabet's square residues with Gödel arithmetic."""
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
EMPTY = '≻⋈∈⊤∋'
ALPHABET = ('0', '1', '2', '3', '4', '5', '6', '7',
            '8', '9', '10', '11', '12', '13', '14', '15')

def field(out, label):
    return next(line.split(None, 1)[1] for line in out.splitlines()
                if line.startswith(label + ' '))

def cells(word):
    return [word[1:-3][i:i + 5] for i in range(0, len(word[1:-3]), 5)]

with (ROOT / 'measurements/godel_hex_square_residues.log').open('a') as record:
    def run(args):
        out = subprocess.run(args, cwd=ROOT, text=True,
                             stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        record.write('COMMAND ' + repr(args) + '\n' + out.stdout +
                     '\nEXIT ' + str(out.returncode) + '\n\n')
        if out.returncode:
            raise RuntimeError(out.stdout)
        return out.stdout

    def decode(parts):
        return field(run(['./godel', 'decode', '⊢' + ''.join(parts) + '⊙⊡⊣']), 'value')

    residues = set()
    for digit in ALPHABET:
        parts = cells(field(run(['./godel', 'encode', digit]), 'word'))
        if digit == '0':
            square = '0'
        else:
            width = max(2, len(parts))
            adapter = decode(parts + [EMPTY] * (2 * width - len(parts)) + parts)
            square = field(run(['./godel', 'frame-op', adapter, str(width), '0',
                                'mul', str(2 * width), '1']), 'result')
        check = run(['./godel', 'product', square, digit, digit])
        assert 'relation.exact-product     PASS' in check
        raw = run(['./run_cmds.sh', 'trilattice_factor read ' + square])
        clean = re.sub(r'\x1b\[[0-9;]*m', '', raw)
        word = next(line.split(':', 1)[1].strip() for line in clean.splitlines()
                    if 'native word      :' in line)
        low = cells(word)[:4]
        low += [EMPTY] * (4 - len(low))
        residue = decode(low)
        residues.add(residue)
        record.write('HEX SQUARE RESIDUE ' + digit + ' -> ' + residue + '\n')
        print(digit, 'square', square, 'low hex value', residue)
    assert residues == {'0', '1', '4', '9'}
    record.write('COMPLETE HEX SQUARE IMAGE {0,1,4,9} PASS\n')
