"""Force cofactor poles for one source-derived odd payload, then check N."""
from pathlib import Path
import runpy
import sys
import subprocess
import gzip
import re
import ast
import argparse

parser=argparse.ArgumentParser()
parser.add_argument('--exchange',action='store_true')
options=parser.parse_args()

HERE=Path(__file__).resolve().parent
saved_argv=sys.argv
replayed=[HERE/'residual_copy_completion_low_pole.md',
          HERE/'residual_copy_completion_low_pole.log.gz']
saved_records={p:p.read_bytes() for p in replayed if p.exists()}
try:
    sys.argv=[str(HERE/'residual_copy_completion.py'),'--anchor-low-pole']
    state=runpy.run_path(sys.argv[0])
finally:
    sys.argv=saved_argv
    for p,contents in saved_records.items():p.write_bytes(contents)
A=dict(state['A'])
B=dict(state['B'])
R=dict(state['corrected'])
normalize,word,op=state['normalize'],state['word'],state['op']
call,field=state['call'],state['field']
logs=state['logs']
native=state['native']
if options.exchange:
    prior=(HERE/'forced_residual_poles.md').read_text()
    prior_a=re.search(r'Payload: `([0-9]+)`',prior).group(1)
    prior_q=re.search(r'Unique bounded cofactor: `([0-9]+)`',prior).group(1)
    def support(value):
        encoded=field(call('encode',value),'word')
        cells=encoded[1:-3]
        return {i//5:1 for i in range(0,len(cells),5)
                if cells[i:i+5]==state['FILLED']}
    A=support(prior_q)
    B=support(prior_a)
    R=dict(ast.literal_eval(re.search(r'Final retained residual: (\[.*\])',prior).group(1)))
assert min(A)==0 and abs(A[0])==1
sign_a=1 if A[max(A)]>0 else -1
ap=word(i for i,c in A.items() if c>0)
an=word(i for i,c in A.items() if c<0)
a_word=op(ap,'sub',an) if sign_a>0 else op(an,'sub',ap)
source_width=len(native[1:-3])//5
a_width=len(a_word[1:-3])//5
bound=source_width-a_width+1
assert bound>0

# Cancel only the actual first residual pole. A[0] makes its coefficient
# unique. No alternative operand prefixes or cofactor candidates are tried.
trace=[]
while R and min(R)<bound:
    position=min(R)
    coefficient=R[position]*A[0]
    B[position]=B.get(position,0)+coefficient
    if not B[position]:del B[position]
    for i,c in A.items():
        R[i+position]=R.get(i+position,0)-coefficient*c
        if not R[i+position]:del R[i+position]
    R,moves=normalize(R)
    assert not R or min(R)>position
    trace.append((position,coefficient,min(R) if R else None,len(R),moves))

# Read the unique nonnegative cofactor within its necessary word-width
# bound. Move every discarded high cofactor term into the residual exactly.
cofactor={i:sign_a*c for i,c in B.items()}
q={}
for position in range(bound):
    incoming=cofactor.get(position,0)
    digit=incoming%2
    carry=(incoming-digit)//2
    if digit:q[position]=1
    if carry:cofactor[position+1]=cofactor.get(position+1,0)+carry
delta=dict(B)
for i,c in q.items():delta[i]=delta.get(i,0)-sign_a*c
delta={i:c for i,c in delta.items() if c}
for i,c in A.items():
    for j,d in delta.items():R[i+j]=R.get(i+j,0)+c*d
R={i:c for i,c in R.items() if c}
R,final_moves=normalize(R)
assert not R or min(R)>=bound
q_word=word(q)
product=op(a_word,'mul',q_word)
rp=word(i for i,c in R.items() if c>0)
rn=word(i for i,c in R.items() if c<0)
assert op(native,'add',rn)==op(product,'add',rp)
factor_check=subprocess.run([str(state['ROOT']/'godel'),'check','mul',a_word,q_word,native],
                            cwd=state['ROOT'],text=True,capture_output=True)
logs.append('FULL SOURCE PRODUCT CHECK\n'+factor_check.stdout+factor_check.stderr+
            '\nEXIT '+str(factor_check.returncode))
closed=product==native and q_word!=word([0]) and a_word!=word([0])
assert ('PASS' in factor_check.stdout) == (product==native)
a_value=field(call('decode',a_word),'value')
q_value=field(call('decode',q_word),'value')
report=['# Forced residual-pole return for the exposed odd payload','',
        'Operand roles exchanged: '+str(options.exchange)+'.','',
        'This is deterministic cofactor arithmetic on one already exposed '
        'operand. It does not enumerate factor supports or supply a new '
        'factor-producing algorithm. Prefix agreement alone is never '
        'accepted as extraction.','',
        f'Source width: {source_width}. Payload width: {a_width}.',
        f'Necessary positive cofactor width bound: {bound}.',
        f'Forced residual-pole cancellations: {len(trace)}.','',
        'Payload: `'+a_value+'`.','',
        'Unique bounded cofactor: `'+q_value+'`.','',
        'Pole/carry trace: '+repr(trace),'',
        'Discarded signed high-cofactor terms: '+repr(sorted(delta.items())),'',
        'Final correction carry trace: '+repr(final_moves),'',
        'Final retained residual: '+repr(sorted(R.items())),'',
        'Full signed source equation: Gödel PASS.',
        'Full source product: '+('PASS' if product==native else 'FAIL')+'.',
        'Proper-factor evidence: '+str(closed)+'.','',
        ('The returned proper pair reconstructs the complete source.' if closed else
         'The complete return rejects this particular odd payload as a '
         'divisor of the source: any positive integer cofactor would fit '
         'the necessary width bound and agree with the uniquely forced '
         'prefix, but its full product differs. This rejects the selected '
         'payload, not source-only extraction generally.')]
read_values=[('payload',a_value),('bounded cofactor',q_value)]
if R:
    residual_sign='+' if R[max(R)]>0 else '-'
    residual_word=op(rp,'sub',rn) if residual_sign=='+' else op(rn,'sub',rp)
    read_values.append(('retained residual '+residual_sign,
                        field(call('decode',residual_word),'value')))
for label,value in read_values:
    command=[str(state['ROOT']/'run_cmds.sh'),'tfactor read '+value]
    reading=subprocess.run(command,cwd=state['ROOT'],capture_output=True,text=True,check=True)
    logs.append(repr(command)+'\n'+reading.stdout)
    clean=re.sub(r'\x1b\[[0-9;]*m','',reading.stdout)
    hex_word=next(line.split(':',1)[1].strip() for line in clean.splitlines()
                  if 'hex-digit word' in line)
    report.extend(['',label+' ordered hex word:','',hex_word])
record_name='forced_residual_poles'+('_exchange' if options.exchange else '')
(HERE/(record_name+'.md')).write_text('\n'.join(report)+'\n')
with gzip.open(HERE/(record_name+'.log.gz'),'wt') as out:
    out.write('\n'.join(logs))
print(f'payload_width={a_width} cofactor_bound={bound} forced_poles={len(trace)} '
      f'residual_first={min(R) if R else None} full_source_product={product==native}',flush=True)
