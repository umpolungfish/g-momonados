"""Apply recorded two-sided partial-copy descent to the exact source return."""
from pathlib import Path
import ast
import re
import subprocess
import gzip

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
logs = []


def load_functions(path, names):
    tree = ast.parse(path.read_text())
    functions = [node for node in tree.body
                 if isinstance(node, ast.FunctionDef) and node.name in names]
    assert {node.name for node in functions} == set(names)
    exec(compile(ast.Module(body=functions, type_ignores=[]), str(path), 'exec'), globals())


load_functions(HERE/'residual_copy_completion.py', ['call', 'field', 'word', 'op', 'normalize'])
load_functions(HERE.parent/'godel_symbolic_partial_absorption.py', ['step'])


def cancel(terms, higher):
    terms = dict(terms)
    moves = []
    while True:
        for i in sorted(terms, reverse=higher):
            c = terms[i]
            if abs(c) >= 2:
                sign = 1 if c > 0 else -1
                terms[i] -= 2*sign
                terms[i+1] = terms.get(i+1, 0)+sign
                move = ('merge', i, sign)
            elif c*terms.get(i+1, 0) < 0:
                sign = 1 if terms[i+1] > 0 else -1
                terms[i+1] -= sign
                terms[i] += 2*sign
                move = ('split-cancel', i, sign)
            else:
                continue
            terms = {j:d for j,d in terms.items() if d}
            moves.append(move)
            break
        else:
            return terms, moves


def magnitude_word(terms):
    normalized, _ = normalize(terms)
    if not normalized:
        return 1, word([])
    sign = normalized[max(normalized)]
    positive = word(i for i,c in normalized.items() if c > 0)
    negative = word(i for i,c in normalized.items() if c < 0)
    return sign, op(positive, 'sub', negative) if sign > 0 else op(negative, 'sub', positive)


def certify(source_word, A, B, R):
    sa, a = magnitude_word(A)
    sb, b = magnitude_word(B)
    nr, _ = normalize(R)
    rp = word(i for i,c in nr.items() if c > 0)
    rn = word(i for i,c in nr.items() if c < 0)
    product = op(a, 'mul', b)
    if sa*sb > 0:
        assert op(source_word, 'add', rn) == op(product, 'add', rp)
    else:
        assert op(op(source_word, 'add', product), 'add', rn) == rp
    return a, b, not nr and sa*sb > 0 and a != word([0]) and b != word([0])


# Recorded multiplicity control: contained partial copies close 21=3*7.
ca, cb, cr = {0:1, 1:1}, {2:1}, {0:1, 1:2, 2:1}
while cr:
    control = step(ca, cb, cr, False)
    assert control is not None
    ca, cb, cr, _ = control
assert certify(field(call('encode', '21'), 'word'), ca, cb, cr)[2]

input_record = HERE/'translated_signed_collection_single_polarity_hex_opposite_continued_e5b3df8bd92d.md'
text = input_record.read_text()
A0 = dict(ast.literal_eval(re.search(r'Signed payload positions: (\[.*\])', text).group(1)))
R0 = dict(ast.literal_eval(re.search(r'Signed residual positions: (\[.*\])', text).group(1)))
d = int(re.search(r'Displacement: ([0-9]+)', text).group(1))
assert 'Translation polarity: -1.' in text
B0 = {0:1, d:-1}
native = field(call('encode', (HERE/'source.txt').read_text().strip()), 'word')
certify(native, A0, B0, R0)
report = ['# Two-sided partial-copy return', '', 'Input record: '+input_record.name+'.',
          'The recorded 21 multiplicity control closes to proper factors through Gödel.', '']
for reverse in (False, True):
    for higher in (False, True):
        A, B, R = dict(A0), dict(B0), dict(R0)
        trace = []
        while True:
            before = sum(abs(c) for c in R.values())
            result = step(A, B, R, reverse)
            move = None
            if result is not None:
                A, B, R, move = result
            R, rewrites = cancel(R, higher)
            if result is None and not rewrites:
                break
            after = sum(abs(c) for c in R.values())
            assert after < before
            certify(native, A, B, R)
            trace.append((before, after, move, rewrites, sorted(A.items()), sorted(B.items()), sorted(R.items())))
        a, b, closed = certify(native, A, B, R)
        residual_magnitude = sum(abs(c) for c in R.values())
        report.extend([f'Shift reverse: {reverse}; cancellation higher-first: {higher}.',
                       f'Accepted rounds: {len(trace)}; residual magnitude: {residual_magnitude}; closed: {closed}.',
                       'Complete return trace: '+repr(trace),
                       'Final operand A: '+repr(sorted(A.items())),
                       'Final operand B: '+repr(sorted(B.items())),
                       'Final correction: '+repr(sorted(R.items())), ''])
        for label, terms in [('operand A', A), ('operand B', B), ('correction', R)]:
            sign, encoded = magnitude_word(terms)
            value = field(call('decode', encoded), 'value')
            reading = subprocess.run([str(ROOT/'run_cmds.sh'), 'tfactor read '+value],
                                     cwd=ROOT, text=True, capture_output=True, check=True)
            logs.append(label+'\n'+reading.stdout)
            clean = re.sub(r'\x1b\[[0-9;]*m', '', reading.stdout)
            hex_word = next(line.split(':', 1)[1].strip() for line in clean.splitlines()
                            if 'hex-digit word' in line)
            report.extend([f'{label} sign {sign}: `{value}`.', hex_word, ''])
        print(f'reverse={reverse} higher={higher} rounds={len(trace)} residual_magnitude={residual_magnitude} closed={closed}', flush=True)
(HERE/'two_sided_partial_return.md').write_text('\n'.join(report)+'\n')
with gzip.open(HERE/'two_sided_partial_return.log.gz', 'wt') as out:
    out.write('\n'.join(logs))
