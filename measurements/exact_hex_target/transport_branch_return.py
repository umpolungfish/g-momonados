"""Retain value-changing transport endpoints and combine further residual operations."""
from pathlib import Path
import ast
import re
import gzip
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
logs = []
preserve_odd = True


def load_functions(path,names):
    tree = ast.parse(path.read_text())
    functions = [n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in names]
    assert {n.name for n in functions} == set(names)
    exec(compile(ast.Module(body=functions,type_ignores=[]),str(path),'exec'),globals())


load_functions(HERE/'residual_copy_completion.py',['call','field','word','op','normalize'])
load_functions(HERE/'two_sided_partial_return.py',['cancel','magnitude_word','certify'])
load_functions(HERE/'combined_zero_partial_return.py',['fast_step','magnitude','expanded','descend'])
step = fast_step
native = field(call('encode',(HERE/'source.txt').read_text().strip()),'word')
parent_text = (HERE/'operand_transport_combined_return.md').read_text()
comparisons = ast.literal_eval(re.search(r'Complete comparisons: (\[.*\])',parent_text).group(1))
wanted = {tuple(row[:6]) for row in comparisons if row[-1]}
previous_b = dict(ast.literal_eval(re.search(r'Final operand B: (\[.*\])',parent_text).group(1)))
excluded_b = normalize(previous_b)[0]
seeds = []
with gzip.open(HERE/'operand_transport_combined_return.log.gz','rt') as archive:
    for line in archive:
        if not line.startswith('TRANSPORT PATH '):
            continue
        # Read only the eight recorded value-changing endpoints.
        prefix = re.match(r'TRANSPORT PATH \((\d+), (\d+), (\d+), (-?\d+), (True|False), (True|False),',line)
        assert prefix
        key = tuple(int(x) for x in prefix.groups()[:4])+tuple(x=='True' for x in prefix.groups()[4:])
        if key not in wanted:
            continue
        row = ast.literal_eval(line[len('TRANSPORT PATH '):])
        A,B,R = (dict(row[i]) for i in (8,9,10))
        assert normalize(B)[0] != excluded_b
        certify(native,A,B,R)
        seeds.append((key,A,B,R))
assert len(seeds)==len(wanted)==8
report = ['# Value-changing transport branches','',
          'Retain all eight placement-changing endpoints, including increased corrections. '
          'They are source-derived operation endpoints, not guessed factor supports.',
          'Every retained parent and endpoint is independently checked through Gödel. '
          'The previous excluded placement remains an explicit comparison word.','']
for key,A0,B0,R0 in seeds:
    expected,_ = normalize(expanded(A0,B0,R0))
    best = None
    checks = []
    # Compose one additional residual zero rewrite with two-sided collection
    # and both cancellation orders on each preserved transport endpoint.
    for position,c in sorted(R0.items()):
        if abs(c) != 1:
            continue
        altered = dict(R0)
        altered[position] = -c
        altered[position+1] = altered.get(position+1,0)+c
        altered = {i:d for i,d in altered.items() if d}
        for reverse in (False,True):
            for higher in (False,True):
                A,B,R,path = descend(dict(A0),dict(B0),dict(altered),reverse,higher)
                assert min(normalize(A)[0])==0 and min(normalize(B)[0])==0
                retained = normalize(B)[0] != excluded_b
                score = magnitude(R)
                checks.append((position,c,reverse,higher,score,retained))
                logs.append('BRANCH PATH '+repr((key,position,c,reverse,higher,path,
                                                sorted(A.items()),sorted(B.items()),sorted(R.items()))))
                if retained and (best is None or score < best[0]):
                    best = score,A,B,R,(position,c,reverse,higher,path)
    if best is None:
        A,B,R = A0,B0,R0
        path = None
    else:
        score,A,B,R,path = best
    a,b,closed = certify(native,A,B,R)
    report.extend(['Parent key: '+repr(key)+'.',
                   f'Parent residual: {magnitude(R0)}; endpoint residual: {magnitude(R)}; closed: {closed}.',
                   'Placement differs from excluded value: '+str(normalize(B)[0] != excluded_b)+'.',
                   'Complete comparison: '+repr(checks),
                   'Selected combined path: '+repr(path),
                   'Endpoint operand A: '+repr(sorted(A.items())),
                   'Endpoint operand B: '+repr(sorted(B.items())),
                   'Endpoint correction: '+repr(sorted(R.items())),''])
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
    print(f'parent={key} parent_residual={magnitude(R0)} endpoint_residual={magnitude(R)} '
          f'placement_retained={normalize(B)[0] != excluded_b} closed={closed}',flush=True)
    (HERE/'transport_branch_return.md').write_text('\n'.join(report)+'\n')
with gzip.open(HERE/'transport_branch_return.log.gz','wt') as out:
    out.write('\n'.join(logs))
