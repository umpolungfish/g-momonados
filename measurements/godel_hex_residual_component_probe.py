"""Carry shared components of exposed hex square operands back to the source."""
from pathlib import Path
import re
import subprocess
import sys
from godel_hex_record_name import record_name, read_record

ROOT = Path(__file__).resolve().parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
source = sys.argv[1]
label = record_name(source)
prior = read_record(ROOT / 'measurements' / ('godel_hex_square_bit_lift_' + label + '_upper_frame.log'))
returns = re.findall(r'^SIGNED SOURCE RETURN ' + re.escape(source) + r' = ([+-])(\d+) \* (\d+) - \(([+-])(\d+)\)$', prior, re.M)
assert returns, 'No exposed source relationship to follow'

with (ROOT / 'measurements' / ('godel_hex_residual_component_' + label + '.log')).open('a') as record:
    def run(args):
        result = subprocess.run(args, cwd=ROOT, capture_output=True, text=True)
        record.write('COMMAND ' + repr(args) + '\n' + result.stdout + result.stderr + '\n')
        if result.returncode:
            raise RuntimeError(result.stdout + result.stderr)
        return result.stdout

    def field(out, name):
        return next(line.split(None, 1)[1] for line in out.splitlines() if line.startswith(name + ' '))

    def encode(value):
        word = field(run(['./godel', 'encode', value]), 'word')
        return [word[1:-3][i:i+5] for i in range(0, len(word[1:-3]), 5)]

    def operate(a, op, b):
        left, right = encode(a), encode(b)
        width = max(2, len(left), len(right))
        composed = field(run(['./godel', 'decode', '⊢' + ''.join(left + [EMPTY] * (2*width-len(left)) + right) + '⊙⊡⊣']), 'value')
        return run(['./godel', 'frame-op', composed, str(width), '0', op, str(2*width), '1'])

    def common(a, b):
        # Every remainder is instrument-produced. No numerical factor candidates.
        while b != '0':
            remainder = field(operate(a, 'divmod', b), 'remainder')
            if remainder != '0':
                assert len(encode(remainder)) <= len(encode(b))
            a, b = b, remainder
        return a

    seen = set()
    for left_sign, left, right, residual_sign, residual in returns:
        for side, operand in (('left', left), ('right', right)):
            if (operand, residual) in seen:
                continue
            seen.add((operand, residual))
            component = common(operand, residual)
            # N = signed operand product - signed residual, so this component
            # must divide N. Verify that implication against the complete source.
            out = operate(source, 'divmod', component)
            assert field(out, 'remainder') == '0'
            cofactor = field(out, 'result')
            certificate = run(['./godel', 'product', source, component, cofactor])
            assert 'relation.exact-product     PASS' in certificate
            proper = component not in {'0', '1', source} and cofactor not in {'0', '1', source}
            record.write('RESIDUAL COMPONENT side=' + side + ' component=' + component + ' cofactor=' + cofactor + ' proper=' + str(proper) + '\n')
            print('residual component', side, component, 'proper=' + str(proper), flush=True)
            if proper:
                run(['./run_cmds.sh', 'tfactor read ' + component])
                run(['./run_cmds.sh', 'tfactor read ' + cofactor])
                print('FACTORS', component, cofactor, flush=True)
                break
        else:
            continue
        break
