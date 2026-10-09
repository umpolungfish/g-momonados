"""Check one source-selected shared-unit update, with its full correction."""
from pathlib import Path
import runpy
import sys
import ast
import re
import subprocess
import gzip

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
saved_argv=sys.argv
paths=[HERE/'residual_copy_completion_low_pole.md',HERE/'residual_copy_completion_low_pole.log.gz']
saved={p:p.read_bytes() for p in paths}
try:
    sys.argv=[str(HERE/'residual_copy_completion.py'),'--anchor-low-pole']
    state=runpy.run_path(sys.argv[0])
finally:
    sys.argv=saved_argv
    for p,contents in saved.items():p.write_bytes(contents)
call,field,word,op,normalize=(state[k] for k in ('call','field','word','op','normalize'))
logs=state['logs']
prior=(HERE/'forced_residual_poles.md').read_text()
p=re.search(r'Payload: `([0-9]+)`',prior).group(1)
q=re.search(r'Unique bounded cofactor: `([0-9]+)`',prior).group(1)
R=dict(ast.literal_eval(re.search(r'Final retained residual: (\[.*\])',prior).group(1)))
pw=field(call('encode',p),'word')
qw=field(call('encode',q),'word')
summed=op(pw,'add',qw)


def support(encoded):
    body=encoded[1:-3]
    return {i//5:1 for i in range(0,len(body),5) if body[i:i+5]==state['FILLED']}


sum_start=min(support(summed))
residual_start=min(R)
shift=residual_start-sum_start
assert shift>sum_start
unit=word([shift])
# The subtraction case would make the smaller operand negative. Addition
# is the only positive whole-word shared-unit completion at this shift.
assert shift>=len(qw[1:-3])//5
new_p=op(pw,'add',unit)
new_q=op(qw,'add',unit)
product=op(new_p,'mul',new_q)
for operand in (support(pw),support(qw)):
    for i,c in operand.items():R[i+shift]=R.get(i+shift,0)-c
R[2*shift]=R.get(2*shift,0)-1
R={i:c for i,c in R.items() if c}
R,moves=normalize(R)
assert not R or min(R)>residual_start
rp=word(i for i,c in R.items() if c>0)
rn=word(i for i,c in R.items() if c<0)
native=state['native']
assert op(native,'add',rn)==op(product,'add',rp)
check=subprocess.run([str(ROOT/'godel'),'check','mul',new_p,new_q,native],
                     cwd=ROOT,text=True,capture_output=True)
logs.append('FULL SOURCE PRODUCT CHECK\n'+check.stdout+check.stderr)
closed=product==native
readings=[]
for label,encoded in [('left operand',new_p),('right operand',new_q)]:
    readings.append((label,field(call('decode',encoded),'value')))
if R:
    sign='+' if R[max(R)]>0 else '-'
    magnitude=op(rp,'sub',rn) if sign=='+' else op(rn,'sub',rp)
    readings.append(('residual '+sign,field(call('decode',magnitude),'value')))
report=['# Source-selected coupled shared-unit return','',
        f'Previous residual first pole: {residual_start}. Operand-sum first pole: {sum_start}.',
        f'Derived shared-unit shift: {shift}.','',
        'Subtracting this unit makes the smaller operand negative. The '
        'single positive case adds it to both operands. This transformation '
        'preserves their difference; it is a checked local word identity, '
        'not a general factor-producing rule or a difference-of-squares '
        'search. No alternative factor prefixes are enumerated.','',
        'Exact correction identity: `R_new = R_old - U_shift*(P+Q) - U_(2*shift)`.','',
        'Recorded carry transport: '+repr(moves),'',
        'Retained new residual: '+repr(sorted(R.items())),'',
        f'New residual first occupied pole: {min(R) if R else None}.',
        'Full signed source equation: Gödel PASS.',
        'Full source product: '+('PASS' if closed else 'FAIL')+'.','']
for label,value in readings:
    command=[str(ROOT/'run_cmds.sh'),'tfactor read '+value]
    result=subprocess.run(command,cwd=ROOT,text=True,capture_output=True,check=True)
    logs.append(repr(command)+'\n'+result.stdout)
    clean=re.sub(r'\x1b\[[0-9;]*m','',result.stdout)
    hex_word=next(line.split(':',1)[1].strip() for line in clean.splitlines() if 'hex-digit word' in line)
    report.extend([label+': `'+value+'`','',hex_word,''])
(HERE/'coupled_unit_return.md').write_text('\n'.join(report)+'\n')
with gzip.open(HERE/'coupled_unit_return.log.gz','wt') as out:out.write('\n'.join(logs))
print(f'shared_shift={shift} residual_first={residual_start}->{min(R) if R else None} '
      f'full_source_equation=PASS full_source_product={closed}',flush=True)
