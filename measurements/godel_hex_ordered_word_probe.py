"""Recover positioned native cells from complete ordered canonical hex words."""
from pathlib import Path
import re
import subprocess
import sys
ROOT = Path(__file__).resolve().parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
for source in sys.argv[1:]:
    with (ROOT / 'measurements' / ('godel_hex_ordered_word_' + source + '.log')).open('a') as record:
        def run(args):
            result = subprocess.run(args, cwd=ROOT, capture_output=True, text=True)
            record.write('COMMAND ' + repr(args) + '\n' + result.stdout + result.stderr + '\n')
            if result.returncode:
                raise RuntimeError(result.stdout + result.stderr)
            return re.sub(r'\x1b\[[0-9;]*m', '', result.stdout)
        out = run(['./run_cmds.sh', 'trilattice_factor read ' + source])
        hex_word = next(line.split(':', 1)[1].strip() for line in out.splitlines() if 'hex-digit word   :' in line)
        native = next(line.split(':', 1)[1].strip() for line in out.splitlines() if 'native word      :' in line)
        motifs = re.findall('⊢[^⊢⊣]*⊣', hex_word)
        assert ''.join(motifs) == hex_word
        parts = []
        for motif in reversed(motifs):
            fork, truth, falsity, info = ('∈' in motif, '⊤' in motif, '⊥' in motif, '⊞' in motif)
            expected = '⊢' + ('∈' if fork else '') + ('⊤' if truth else '') + '≻⋈' + ('⊥' if falsity else '') + '≺' + ('⋈' if fork else '') + ('⊞' if info else '') + ('∋' if fork else '') + '⊙⊡⊣'
            assert expected == motif, 'Ordered motif differs from canonical composition'
            parts.extend(FILLED if slot else EMPTY for slot in (truth, falsity, info, fork))
            if fork and info:
                assert motif.index('≺') < motif.index('⊞') < motif.index('∋')
                record.write('BANKED INFORMATION ORDER ' + motif + ' PASS\n')
        while len(parts) > 1 and parts[-1] == EMPTY:
            parts.pop()
        recovered = '⊢' + ''.join(parts) + '⊙⊡⊣'
        assert recovered == native
        decoded = run(['./godel', 'decode', recovered])
        value = next(line.split(None, 1)[1] for line in decoded.splitlines() if line.startswith('value '))
        assert value == source
        record.write('ORDERED HEX SOURCE RETURN exact=PASS\n')
        print(source, 'ordered hex/native/source return PASS')
