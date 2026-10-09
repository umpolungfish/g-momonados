"""Transport operand zero rewrites before collection, retaining odd operands."""
from pathlib import Path
import ast
import re
import subprocess
import gzip
import argparse

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--complete-return', type=int, choices=(0, 1),
                    help='Use a complete return of an observed transport placement as the seed.')
options = parser.parse_args()
record_name = 'operand_transport_combined_return'+(
    '' if options.complete_return is None else '_complete_'+str(options.complete_return))

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
logs = []
preserve_odd = True


def load_functions(path, names):
    tree = ast.parse(path.read_text())
    functions = [node for node in tree.body
                 if isinstance(node, ast.FunctionDef) and node.name in names]
    assert {node.name for node in functions} == set(names)
    exec(compile(ast.Module(body=functions,type_ignores=[]),str(path),'exec'),globals())


load_functions(HERE/'residual_copy_completion.py',['call','field','word','op','normalize'])
load_functions(HERE/'two_sided_partial_return.py',['cancel','magnitude_word','certify'])
load_functions(HERE/'combined_zero_partial_return.py',['fast_step','magnitude','expanded','descend'])
step = fast_step
parent = HERE/'combined_zero_partial_return_overlap_low_pole_odd.md'
text = parent.read_text()
A,B,R = (dict(ast.literal_eval(re.search(label+r': (\[.*\])',text).group(1)))
         for label in ('Final operand A','Final operand B','Final correction'))
prelude = None
if options.complete_return is not None:
    parent = HERE/'transport_placement_returns.md'
    section = parent.read_text().split('Branch indexes: ')[1:][options.complete_return]
    numerals = [re.search(r'^'+label+r' word: (.*)$', section, re.M).group(1)
                for label in ('quotient', 'placement', 'remainder')]
    A,B,R = ({i:1 for i in range(len(n[1:-3])//5)
              if n[1:-3][5*i:5*i+5] == FILLED} for n in numerals)
    if A.get(0,0) % 2 == 0:
        assert B.get(0)==R.get(0)==1
        A[0] = 1
        for i,c in B.items():
            R[i] = R.get(i,0)-c
            if not R[i]:
                del R[i]
        prelude = 'The remainder low pole completes the quotient low pole; the full placement copy remains in the correction.'
    assert min(normalize(A)[0]) == min(normalize(B)[0]) == 0
native = field(call('encode',(HERE/'source.txt').read_text().strip()),'word')
certify(native,A,B,R)
expected,_ = normalize(expanded(A,B,R))
print(f'seed_return=PASS residual={magnitude(R)} operand_terms=({len(A)},{len(B)})', flush=True)
rounds,comparisons = [],[]
checked,changed_placement = 0,0
while R:
    score = magnitude(R)
    previous_b = normalize(B)[0]
    best = None
    for side,payload in enumerate((A,B)):
        other = B if side == 0 else A
        for position,c in sorted(payload.items()):
            if abs(c) != 1:
                continue
            altered = dict(payload)
            altered[position] = -c
            altered[position+1] = altered.get(position+1,0)+c
            altered = {i:d for i,d in altered.items() if d}
            # D=c*U_(i+1)-2c*U_i returns zero. Retain the full -D*other correction.
            transported = dict(R)
            for i,d in ((position,-2*c),(position+1,c)):
                for j,e in other.items():
                    transported[i+j] = transported.get(i+j,0)-d*e
            transported = {i:d for i,d in transported.items() if d}
            left,right = (altered,B) if side == 0 else (A,altered)
            for reverse in (False,True):
                for higher in (False,True):
                    a,b,r,path = descend(dict(left),dict(right),dict(transported),reverse,higher)
                    assert min(normalize(a)[0]) == 0 and min(normalize(b)[0]) == 0
                    changed = normalize(b)[0] != previous_b
                    checked += 1
                    changed_placement += changed
                    final = magnitude(r)
                    if checked == 1 or checked % 64 == 0:
                        print(f'checked={checked} changed_placement={changed_placement} '
                              f'current_endpoint_residual={final}', flush=True)
                    comparisons.append((len(rounds),side,position,c,reverse,higher,final,changed))
                    logs.append('TRANSPORT PATH '+repr((len(rounds),side,position,c,reverse,higher,
                                                        sorted(transported.items()),path,
                                                        sorted(a.items()),sorted(b.items()),sorted(r.items()))))
                    # This placement has an independently verified nonzero source remainder.
                    # Retain unchanged-placement paths, but do not accept them as a replacement.
                    if not changed or final >= score or (best is not None and final >= best[0]):
                        continue
                    best = final,a,b,r,(side,position,c,reverse,higher,path)
                    if checked % 64 == 0:
                        print(f'checked={checked} changed_placement_paths={changed_placement}',flush=True)
    if best is None:
        break
    final,A,B,R,path = best
    a,b,closed = certify(native,A,B,R)
    rounds.append((score,final,path,sorted(A.items()),sorted(B.items()),sorted(R.items())))
    (HERE/(record_name+'_checkpoint.md')).write_text(
        '# Certified operand-transport checkpoint\n\n'
        'Parent: '+parent.name+'.\n'
        'Complete-return seed index: '+repr(options.complete_return)+'.\n'
        'Accepted rounds: '+repr(rounds)+'\n'
        'Final operand A: '+repr(sorted(A.items()))+'\n'
        'Final operand B: '+repr(sorted(B.items()))+'\n'
        'Final correction: '+repr(sorted(R.items()))+'\n')
    print(f'accepted={len(rounds)} residual={score}->{final} placement_changed=True closed={closed}',flush=True)
    if closed:
        break
a,b,closed = certify(native,A,B,R)
report = ['# Operand-transport combined return','',
          'Parent: '+parent.name+'.',
          'Complete-return seed index: '+repr(options.complete_return)+'.',
          'Low-pole completion: '+repr(prelude)+'.',
          'Operand polarity rewrites transport their complete correlated zero correction. '
          'Partial collection precedes cancellation; both orders are retained; operand values remain odd.',
          'Every tested endpoint reconstructs the complete source dictionary. '
          'Every accepted endpoint additionally passes native Gödel.',
          'Unchanged-placement paths are retained but not accepted as a replacement for the excluded placement.',
          f'Checked paths: {checked}; placement-changing endpoints: {changed_placement}; accepted rounds: {len(rounds)}.',
          f'Final correction magnitude: {magnitude(R)}; closed: {closed}.','',
          'Complete comparisons: '+repr(comparisons),'',
          'Complete accepted paths: '+repr(rounds),'',
          'Final operand A: '+repr(sorted(A.items())),
          'Final operand B: '+repr(sorted(B.items())),
          'Final correction: '+repr(sorted(R.items())),'']
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
(HERE/(record_name+'.md')).write_text('\n'.join(report)+'\n')
with gzip.open(HERE/(record_name+'.log.gz'),'wt') as out:
    out.write('\n'.join(logs))
print(f'checked={checked} changed_placement={changed_placement} accepted={len(rounds)} '
      f'residual={magnitude(R)} closed={closed}',flush=True)
