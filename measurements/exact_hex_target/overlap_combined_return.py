"""Construct overlapping source seeds and combine two-sided residual operations."""
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


load_functions(HERE/'residual_copy_completion.py', ['call','field','word','op','normalize'])
load_functions(HERE/'two_sided_partial_return.py', ['cancel','magnitude_word','certify'])
load_functions(HERE/'combined_zero_partial_return.py', ['fast_step'])


def expanded(a,b):
    result = {}
    for i,c in a.items():
        for j,d in b.items():
            result[i+j] = result.get(i+j,0)+c*d
    return {i:c for i,c in result.items() if c}


text = (HERE/'translated_signed_collection_higher_first.md').read_text()
source = dict(ast.literal_eval(re.search(r'Complete signed source positions: (\[.*\])',text).group(1)))
native = field(call('encode',(HERE/'source.txt').read_text().strip()),'word')
certify(native, {0:1}, source, {})
positions = sorted(source)
seeds = []
for offset in sorted({j-i for i in positions for j in positions if j>i}):
    for polarity in (1,-1):
        matches = [i for i in positions if source.get(i+offset) == polarity*source[i]]
        if len(matches)<2:
            continue
        overlap = set(matches)&{i+offset for i in matches}
        if not overlap:
            continue
        anchor = min(matches)
        A = {i-anchor:source[i] for i in matches}
        B = {anchor:1,anchor+offset:polarity}
        product = expanded(A,B)
        R = {i:source.get(i,0)-product.get(i,0) for i in set(source)|set(product)}
        R = {i:c for i,c in R.items() if c}
        restored = dict(product)
        for i,c in R.items():
            restored[i] = restored.get(i,0)+c
        assert {i:c for i,c in restored.items() if c} == source
        seeds.append((len(A),anchor,offset,polarity,A,B,R,sorted(overlap)))
assert seeds
seeds.sort(key=lambda row:row[:4])
# Source-support selection only: smallest observed overlapping payload,
# then its earliest source anchor, displacement and polarity.
size,anchor,offset,polarity,A0,B0,R0,overlap = seeds[0]
certify(native,A0,B0,R0)
report = ['# Overlap-seeded combined return','',
          'Every seed is an observed overlapping translated source payload '
          'with all multiplicity corrections retained.',
          'Selection uses payload term count and source positions only.',
          f'Observed seeds: {len(seeds)}.',
          f'Selected payload terms: {size}; anchor: {anchor}; displacement: {offset}; polarity: {polarity}.',
          'Source seed census: '+repr([(s,a,d,p,sorted(x.items()),sorted(y.items()),sorted(r.items()),o)
                                       for s,a,d,p,x,y,r,o in seeds]),'']
for reverse in (False,True):
    for higher in (False,True):
        A,B,R = dict(A0),dict(B0),dict(R0)
        trace = []
        certify(native,A,B,R)
        while True:
            before = sum(abs(c) for c in R.values())
            result = fast_step(A,B,R,reverse)
            move = None
            if result is not None:
                A,B,R,move = result
            R,rewrites = cancel(R,higher)
            if result is None and not rewrites:
                break
            after = sum(abs(c) for c in R.values())
            assert after < before
            certify(native,A,B,R)
            trace.append((before,after,move,rewrites,sorted(A.items()),sorted(B.items()),sorted(R.items())))
        a,b,closed = certify(native,A,B,R)
        achanged = normalize(A)[0] != normalize(A0)[0]
        bchanged = normalize(B)[0] != normalize(B0)[0]
        magnitude = sum(abs(c) for c in R.values())
        report.extend([f'Shift reverse: {reverse}; cancellation higher-first: {higher}.',
                       f'Rounds: {len(trace)}; residual magnitude: {magnitude}; closed: {closed}.',
                       f'Operand A changed value: {achanged}; operand B changed value: {bchanged}.',
                       'Complete combined trace: '+repr(trace),
                       'Final operand A: '+repr(sorted(A.items())),
                       'Final operand B: '+repr(sorted(B.items())),
                       'Final correction: '+repr(sorted(R.items())), ''])
        for label,terms in [('operand A',A),('operand B',B),('correction',R)]:
            sign,encoded = magnitude_word(terms)
            value = field(call('decode',encoded),'value')
            reading = subprocess.run([str(ROOT/'run_cmds.sh'),'tfactor read '+value],
                                     cwd=ROOT,text=True,capture_output=True,check=True)
            logs.append(label+'\n'+reading.stdout)
            clean = re.sub(r'\x1b\[[0-9;]*m','',reading.stdout)
            hex_word = next(line.split(':',1)[1].strip() for line in clean.splitlines()
                            if 'hex-digit word' in line)
            report.extend([f'{label} sign {sign}: `{value}`.',hex_word,''])
        print(f'offset={offset} polarity={polarity} rounds={len(trace)} residual={magnitude} '
              f'A_changed={achanged} B_changed={bchanged} closed={closed}',flush=True)
(HERE/'overlap_combined_return.md').write_text('\n'.join(report)+'\n')
with gzip.open(HERE/'overlap_combined_return.log.gz','wt') as out:
    out.write('\n'.join(logs))
