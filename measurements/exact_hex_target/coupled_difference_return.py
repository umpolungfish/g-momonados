"""Check the unique bounded complementary return of the current difference."""
from pathlib import Path
import runpy
import subprocess
import gzip
import re

HERE=Path(__file__).resolve().parent
saved={p:p.read_bytes() for p in [HERE/'coupled_unit_return.md',HERE/'coupled_unit_return.log.gz']}
try:
    state=runpy.run_path(str(HERE/'coupled_unit_return.py'))
finally:
    for p,contents in saved.items():p.write_bytes(contents)
word,op,call,field,normalize,support=(state[k] for k in ('word','op','call','field','normalize','support'))
logs=state['logs']
pw,qw=state['new_p'],state['new_q']
R=dict(state['R'])
native=state['native']
assert R and R[max(R)]>0
summed=op(pw,'add',qw)
sum_support=support(summed)
r=min(R)
k=min(sum_support)
modulus_width=r-k
delta_bound=len(native[1:-3])//5-len(pw[1:-3])//5+1
assert r>2*k and modulus_width>delta_bound
low_sum=word(i for i in sum_support if i<modulus_width)
delta=op(word([modulus_width]),'sub',low_sum)
delta_width=len(delta[1:-3])//5
new_p=op(pw,'add',delta)
new_q=op(qw,'add',delta)
product=op(new_p,'mul',new_q)
delta_support=support(delta)
for i in delta_support:
    for j in sum_support:R[i+j]=R.get(i+j,0)-1
    for j in delta_support:R[i+j]=R.get(i+j,0)-1
R={i:c for i,c in R.items() if c}
R,moves=normalize(R)
assert not R or min(R)>=r
rp=word(i for i,c in R.items() if c>0)
rn=word(i for i,c in R.items() if c<0)
assert op(native,'add',rn)==op(product,'add',rp)
check=subprocess.run([str(state['ROOT']/'godel'),'check','mul',new_p,new_q,native],
                     cwd=state['ROOT'],text=True,capture_output=True)
logs.append('FULL SOURCE PRODUCT CHECK\n'+check.stdout+check.stderr)
closed=product==native
assert ('PASS' in check.stdout)==closed
report=['# Complete return of the exposed operand difference','',
        f'Source residual pole: {r}. Operand-sum pole: {k}.',
        f'Complement modulus word width: {modulus_width}.',
        f'Necessary positive shared increment bound: less than U_{delta_bound}.',
        f'Computed complementary increment width: {delta_width}.','',
        'For a positive pair with this same difference, both operands must '
        'change by the same integer d. Since the existing product is below '
        'the source, d is positive. The source and larger-operand widths '
        'bound d strictly below the indicated unit.','',
        'The exact equation is d*(d+S)=R. If the first pole of d is below '
        'the first pole of S, the product pole is twice that lower pole '
        'and cannot reach the observed R pole. If it is above the S pole, '
        'd must begin at or above the complement modulus and violates '
        'the positive width bound. If it equals the S pole, d+S must '
        'vanish below the complement modulus. Therefore d is the unique '
        'positive representative of -S at that modulus.','',
        'That representative is read from the complete low sum word and '
        'checked through Gödel. No alternative factor supports or square '
        'candidates are enumerated.','',
        'Increment within necessary width bound: '+str(delta_width<=delta_bound)+'.',
        'Full signed source equation: Gödel PASS.',
        'Full source product: '+('PASS' if closed else 'FAIL')+'.',
        f'Retained correction first pole: {min(R) if R else None}.','',
        'Retained signed correction: '+repr(sorted(R.items())),'',
        'Recorded carry transport: '+repr(moves),'',
        ('The complete difference-preserving return reconstructs the source.' if closed else
         'The complete check rules out this exposed operand difference '
         'for a proper positive source pair. Any successful next '
         'transformation must alter the difference as well as the operands. '
         'This does not rule out extraction under another source-derived '
         'relationship.')]
readings=[('complementary increment',delta),('new left operand',new_p),('new right operand',new_q)]
if R:
    sign='+' if R[max(R)]>0 else '-'
    magnitude=op(rp,'sub',rn) if sign=='+' else op(rn,'sub',rp)
    readings.append(('retained residual '+sign,magnitude))
for label,encoded in readings:
    value=field(call('decode',encoded),'value')
    command=[str(state['ROOT']/'run_cmds.sh'),'tfactor read '+value]
    reading=subprocess.run(command,cwd=state['ROOT'],capture_output=True,text=True,check=True)
    logs.append(repr(command)+'\n'+reading.stdout)
    clean=re.sub(r'\x1b\[[0-9;]*m','',reading.stdout)
    hex_word=next(line.split(':',1)[1].strip() for line in clean.splitlines() if 'hex-digit word' in line)
    report.extend(['',label+': `'+value+'`','',hex_word])
(HERE/'coupled_difference_return.md').write_text('\n'.join(report)+'\n')
with gzip.open(HERE/'coupled_difference_return.log.gz','wt') as out:out.write('\n'.join(logs))
print(f'modulus_width={modulus_width} delta_width={delta_width} bound={delta_bound} '
      f'residual_first={min(R) if R else None} full_source_product={closed}',flush=True)
