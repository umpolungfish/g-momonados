"""Carry-preserving cyclic returns of complete source hex motif blocks."""
from pathlib import Path
import gzip
import re
import subprocess
import sys
from godel_hex_record_name import record_name

ROOT = Path(__file__).resolve().parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
source = sys.argv[1]
label = record_name(source)

with gzip.open(ROOT / 'measurements' / ('godel_hex_cyclic_fold_' + label + '.log.gz'), 'at') as record:
    def run(args, underflow=False):
        out = subprocess.run(args, cwd=ROOT, capture_output=True, text=True)
        record.write('COMMAND ' + repr(args) + '\n' + out.stdout + out.stderr + '\n')
        if out.returncode:
            if underflow and 'frame subtraction underflow' in out.stderr + out.stdout:
                return None
            raise RuntimeError(out.stderr + out.stdout)
        return out.stdout

    def field(out, name):
        return next(line.split(None, 1)[1] for line in out.splitlines() if line.startswith(name + ' '))

    def cells(word):
        return [word[1:-3][i:i+5] for i in range(0, len(word[1:-3]), 5)]

    def encode(value):
        return cells(field(run(['./godel', 'encode', value]), 'word'))

    def decode(parts):
        return field(run(['./godel', 'decode', '⊢' + ''.join(parts or [EMPTY]) + '⊙⊡⊣']), 'value')

    def operate(a, op, b):
        if b == '0' and op in {'add', 'sub'}:
            return 'result ' + a + '\n'
        left, right = encode(a), encode(b)
        width = max(2, len(left), len(right))
        combined = decode(left + [EMPTY] * (2*width-len(left)) + right)
        return run(['./godel', 'frame-op', combined, str(width), '0', op, str(2*width), '1'], underflow=(op == 'sub'))

    def common(a, b):
        while b != '0':
            a, b = b, field(operate(a, 'divmod', b), 'remainder')
        return a

    read = re.sub(r'\x1b\[[0-9;]*m', '', run(['./run_cmds.sh', 'tfactor read ' + source]))
    native = next(line.split(':', 1)[1].strip() for line in read.splitlines() if 'native word      :' in line)
    hex_word = next(line.split(':', 1)[1].strip() for line in read.splitlines() if 'hex-digit word   :' in line)
    motifs = re.findall('⊢[^⊢⊣]*⊣', hex_word)
    assert ''.join(motifs) == hex_word
    positioned = []
    for motif in reversed(motifs):
        positioned.extend(FILLED if mark in motif else EMPTY for mark in ('⊤', '⊥', '⊞', '∈'))
    while len(positioned) > 1 and positioned[-1] == EMPTY:
        positioned.pop()
    assert positioned == cells(native)
    assert len(positioned) >= 100, "Source must have at least 100 native cells"
    assert decode(positioned) == source
    motif_width = len(motifs) // 2
    found = False
    while motif_width * 4 >= 100 and not found:
        width = motif_width * 4
        blocks = [decode(positioned[start:start+width]) for start in range(0, len(positioned), width)]
        for polarity in ('-', '+'):
            # x=+1 or x=-1 is a complete-word return of the block polynomial.
            # Keep the fold signed, rather than discarding carries in digit counts.
            positive, negative = '0', '0'
            for index, block in enumerate(blocks):
                if polarity == '+' and index % 2:
                    negative = field(operate(negative, 'add', block), 'result')
                else:
                    positive = field(operate(positive, 'add', block), 'result')
            difference = operate(positive, 'sub', negative)
            negative_fold = difference is None
            if negative_fold:
                difference = operate(negative, 'sub', positive)
            magnitude = field(difference, 'result')
            translation = decode([EMPTY] * width + [FILLED])
            payload = field(operate(translation, 'sub' if polarity == '-' else 'add', '1'), 'result')
            fold_remainder = field(operate(magnitude, 'divmod', payload), 'remainder')
            if negative_fold and fold_remainder != '0':
                fold_remainder = field(operate(payload, 'sub', fold_remainder), 'result')
            source_return = operate(source, 'divmod', payload)
            assert fold_remainder == field(source_return, 'remainder')
            component = common(payload, fold_remainder)
            certificate = operate(source, 'divmod', component)
            assert field(certificate, 'remainder') == '0'
            cofactor = field(certificate, 'result')
            proof = run(['./godel', 'product', source, component, cofactor])
            assert 'relation.exact-product     PASS' in proof
            proper = component not in {'0', '1', source} and cofactor not in {'0', '1', source}
            record.write('CYCLIC FOLD width=' + str(width) + ' polarity=' + polarity + ' signed-fold=' + ('-' if negative_fold else '+') + magnitude + ' remainder=' + fold_remainder + ' component=' + component + ' cofactor=' + cofactor + ' proper=' + str(proper) + '\n')
            print('cyclic fold', width, polarity, 'component', component, 'proper', proper, flush=True)
            if proper:
                for factor in (component, cofactor):
                    run(['./run_cmds.sh', 'tfactor read ' + factor])
                print('FACTORS', component, cofactor, flush=True)
                found = True
                break
        motif_width //= 2
