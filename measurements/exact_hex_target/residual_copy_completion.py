"""Complete an observed partial copy by retaining its opposite correction."""
from pathlib import Path
import ast
import re
import subprocess
import gzip

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
EMPTY,FILLED='≻⋈∈⊤∋','≻⋈∈⊥∋'
logs=[]


def call(*args):
    result=subprocess.run([str(ROOT/'godel'),*args],cwd=ROOT,
                          capture_output=True,text=True,check=True)
    logs.append(repr(args)+'\n'+result.stdout)
    return result.stdout


def field(text,name):
    return next(line.split(None,1)[1] for line in text.splitlines()
                if re.match('^'+re.escape(name)+r'\s',line))


def word(positions):
    positions=set(positions)
    return '⊢'+''.join(FILLED if i in positions else EMPTY
                      for i in range(max(positions,default=0)+1))+'⊙⊡⊣'


def op(a,operator,b):
    if b==word([]):
        return word([]) if operator=='mul' else a
    ac,bc=a[1:-3],b[1:-3]
    width=max(2,len(ac)//5,len(bc)//5)
    carrier=field(call('decode','⊢'+ac+EMPTY*(2*width-len(ac)//5)+bc+'⊙⊡⊣'),'value')
    result=field(call('frame-op',carrier,str(width),'0',operator,str(2*width),'1'),'result')
    out=field(call('encode',result),'word')
    check=(call('check','add',b,out,a) if operator=='sub'
           else call('check',operator,a,b,out))
    assert 'PASS' in check and 'FAIL' not in check
    return out


def normalize(terms):
    terms=dict(terms)
    moves=[]
    position=0
    while position<=max(terms,default=0):
        incoming=terms.get(position,0)
        output=(0 if incoming%2==0 else
                1 if (incoming+2*terms.get(position+1,0))%4==1 else -1)
        carry=(incoming-output)//2
        if output:terms[position]=output
        else:terms.pop(position,None)
        if carry:
            terms[position+1]=terms.get(position+1,0)+carry
            if not terms[position+1]:del terms[position+1]
            moves.append((position,incoming,output,carry))
        position+=1
        assert position<1200
    return terms,moves


text=(HERE/'translated_signed_collection_hex_opposite_184.md').read_text()
A=dict(ast.literal_eval(re.search(r'Signed payload positions: (\[.*\])',text).group(1)))
R=dict(ast.literal_eval(re.search(r'Signed residual positions: (\[.*\])',text).group(1)))
options=[]
for position in R:
    shift=position-min(A)
    if shift<0 or shift%4 or max(A)+shift>max(R):continue
    for sign in (1,-1):
        matches=[i+shift for i,c in A.items() if R.get(i+shift)==sign*c]
        options.append((len(matches),shift,sign))
matched,shift,sign=max(options)
completion_polarity=sign
copy={i+shift:sign*c for i,c in A.items()}
missing={i:c for i,c in copy.items() if R.get(i)!=c}
corrected=dict(R)
for i,c in copy.items():corrected[i]=corrected.get(i,0)-c
corrected={i:c for i,c in corrected.items() if c}
raw_corrected=dict(corrected)
corrected,moves=normalize(corrected)
B={0:1,184:-1}
B[shift]=B.get(shift,0)+sign
B={i:c for i,c in B.items() if c}

# N=A*B+R remains exact after R=copy+(R-copy). This is a completed
# observed partial copy; the missing terms were NOT claimed present in R.
source=(HERE/'source.txt').read_text().strip()
native=field(call('encode',source),'word')
ap=word(i for i,c in A.items() if c>0)
an=word(i for i,c in A.items() if c<0)
bp=word(i for i,c in B.items() if c>0)
bn=word(i for i,c in B.items() if c<0)
rp=word(i for i,c in corrected.items() if c>0)
rn=word(i for i,c in corrected.items() if c<0)
positive_product=op(op(ap,'mul',bp),'add',op(an,'mul',bn))
negative_product=op(op(an,'mul',bp),'add',op(ap,'mul',bn))
lhs=op(op(native,'add',negative_product),'add',rn)
rhs=op(positive_product,'add',rp)
assert lhs==rhs
readings=[]
for label,terms in [('payload',A),('cofactor',B),('corrected residual',corrected)]:
    positive=word(i for i,c in terms.items() if c>0)
    negative=word(i for i,c in terms.items() if c<0)
    sign='+' if terms[max(terms)]>0 else '-'
    magnitude=op(positive,'sub',negative) if sign=='+' else op(negative,'sub',positive)
    value=field(call('decode',magnitude),'value')
    command=[str(ROOT/'run_cmds.sh'),'tfactor read '+value]
    result=subprocess.run(command,cwd=ROOT,capture_output=True,text=True,check=True)
    logs.append(repr(command)+'\n'+result.stdout)
    clean=re.sub(r'\x1b\[[0-9;]*m','',result.stdout)
    hex_word=next(line.split(':',1)[1].strip() for line in clean.splitlines()
                  if 'hex-digit word' in line)
    readings.append((label,sign,value,hex_word))
report=['# Residual-guided partial-copy completion','',
        'The closest observed translated signed payload is selected by '
        'literal matching at whole-hex boundaries. Missing terms are '
        'introduced with their exact opposite correction; none is discarded.','',
        f'Observed matched terms: {matched} of {len(A)}.',
        f'Selected displacement: {shift}; polarity: {completion_polarity}.',
        'Missing/unequal terms: '+repr(sorted(missing.items())),'',
        'Raw opposite correction: '+repr(sorted(raw_corrected.items())),'',
        'Carry transport trace: '+repr(moves),'',
        'Collected cofactor terms: '+repr(sorted(B.items())),'',
        'Retained corrected residual: '+repr(sorted(corrected.items())),'',
        f'Residual term count: {len(R)} -> {len(corrected)}.',
        'Complete source equation: Gödel PASS.','',
        'The completed copy changes the cofactor and its residual together. '
        'The residual remains nonzero; this is not a proper-factor certificate.','']
for label,sign,value,hex_word in readings:
    report.extend([label+': `'+sign+value+'`','',hex_word,''])
(HERE/'residual_copy_completion.md').write_text('\n'.join(report)+'\n')
with gzip.open(HERE/'residual_copy_completion.log.gz','wt') as out:
    out.write('\n'.join(logs))
print(f'matched={matched}/{len(A)} shift={shift} '
      f'residual_terms={len(R)}->{len(corrected)} carries={len(moves)} '
      'full_source_equation=PASS',flush=True)
