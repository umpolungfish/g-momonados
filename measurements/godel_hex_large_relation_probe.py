"""Keep source-only relation research on numerals of at least 100 bits."""
from pathlib import Path
import subprocess
import sys
ROOT = Path(__file__).resolve().parents[1]
record = ROOT / 'measurements' / 'godel_hex_large_relation_preparation.log'
with record.open('a') as log:
    def run(args):
        result = subprocess.run(args, cwd=ROOT, capture_output=True, text=True)
        log.write('COMMAND ' + repr(args) + '\n' + result.stdout + result.stderr + '\n')
        log.flush()
        if result.returncode:
            raise RuntimeError(result.stdout + result.stderr)
        return result.stdout
    if len(sys.argv) > 1:
        source = sys.argv[1]
    else:
        source_word = (ROOT / 'measurements/ququart/increasing_20261004/160/source.imasm').read_text().strip()
        decoded = run(['./godel', 'decode', source_word])
        source = next(line.split(None, 1)[1] for line in decoded.splitlines() if line.startswith('value '))
    encoded = run(['./godel', 'encode', source])
    width = next(line.split()[-1] for line in encoded.splitlines() if line.startswith('binary.frames '))
    if int(width) < 100:
        raise SystemExit('Relation research requires a source of at least 100 bits')
    log.write('SOURCE-ONLY LARGE RELATION source=' + source + ' bits=' + width + '\n')
    print('source', source, 'bits', width, flush=True)
    followed = run(['python3', 'measurements/godel_hex_square_bit_lift_probe.py', source, '--upper-frame'])
    print(followed.rstrip())
