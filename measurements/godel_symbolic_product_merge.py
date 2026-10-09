"""Merge retained product blocks while preserving and cancelling cross terms."""
from godel_markdown_records import read_record, write_record
import json
from pathlib import Path
from godel_symbolic_boundary_collect import value


def add(*terms):
    out={}
    for t in terms:
        for i,c in t.items():
            out[i]=out.get(i,0)+c
    return {i:c for i,c in out.items() if c}


def product(a,b):
    out={}
    for i,c in a.items():
        for j,d in b.items():
            out[i+j]=out.get(i+j,0)+c*d
    return {i:c for i,c in out.items() if c}


def negate(t):
    return {i:-c for i,c in t.items()}


def cancel(t):
    t=dict(t)
    trace=[]
    while True:
        indices=[i for i,c in t.items() if abs(c)>=2]
        if not indices:
            break
        i=min(indices)
        sign=1 if t[i]>0 else -1
        t[i]-=2*sign
        t[i+1]=t.get(i+1,0)+sign
        t={j:c for j,c in t.items() if c}
        trace.append([i,sign])
    return t,trace


root=Path(__file__).parent
chains=read_record(root/'godel_symbolic_residual_chains')
reports=[]
for row in chains['reports']:
    blocks=[(dict(p['payload']),dict(p['placement'])) for p in row['products']]
    source=dict(row['sourceSignedWord'])
    merges=[]
    for i,(A,B) in enumerate(blocks):
        for j in range(i+1,len(blocks)):
            C,D=blocks[j]
            for swap in (False,True):
                if swap:
                    C,D=D,C
                residual=add(source,negate(product(A,B)),negate(product(C,D)))
                crossLeft,crossRight=product(A,D),product(C,B)
                correction=add(residual,negate(crossLeft),negate(crossRight))
                normalized,moves=cancel(correction)
                left,right=add(A,C),add(B,D)
                assert value(left)*value(right)+value(normalized)==int(row['source'])
                proper=abs(value(left))>1 and abs(value(right))>1
                merges.append(dict(productIndices=[i,j],swapSecond=swap,
                                    left=sorted(left.items()),right=sorted(right.items()),
                                    correction=sorted(normalized.items()),mergeMoves=moves,
                                    factorEvidence='True' if not normalized and proper else 'None'))
    reports.append(dict(source=row['source'],sourceSignedWord=row['sourceSignedWord'],merges=merges))
output=dict(identity='A*B+C*D+R=(A+C)*(B+D)+(R-A*D-C*B)',reports=reports)
write_record(root/'godel_symbolic_product_merges', output)
print(json.dumps([dict(source=r['source'],merges=len(r['merges']),closed=sum(m['factorEvidence']=='True' for m in r['merges']),
                       smallestCorrectionTerms=min((len(m['correction']) for m in r['merges']),default=None)) for r in reports]))
