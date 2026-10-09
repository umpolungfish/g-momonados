"""Source-only signed unit rewrites and literal common-payload collection."""
from godel_markdown_records import read_record, write_record
import json
from pathlib import Path
from godel_run_tiling_probe import intervals


def value(t):
    return sum(c*(1 << i) for i,c in t.items())


def read(n, higher_first=True):
    terms={}
    for lo,hi in intervals(n,True):
        terms[hi]=terms.get(hi,0)+1
        terms[lo]=terms.get(lo,0)-1
    steps=[]
    while True:
        change=False
        # Work from higher positions so the low unit remains an explicit anchor.
        for i in sorted(terms,reverse=higher_first):
            c=terms.get(i,0)
            if abs(c)>=2:
                sign=1 if c>0 else -1
                terms[i]-=2*sign
                terms[i+1]=terms.get(i+1,0)+sign
                rule=f'merge {sign:+d}U_{i} {sign:+d}U_{i} to {sign:+d}U_{i+1}'
            elif c and terms.get(i+1,0)*c<0:
                # Split the higher unit; one contribution cancels the lower.
                sign=1 if terms[i+1]>0 else -1
                terms[i+1]-=sign
                terms[i]+=2*sign
                rule=f'split {sign:+d}U_{i+1}; cancel against {-sign:+d}U_{i}'
            else:
                continue
            terms={j:c for j,c in terms.items() if c}
            assert value(terms)==n
            steps.append(dict(rule=rule,terms=sorted(terms.items())))
            change=True
            break
        if not change:
            break
    returns=[]
    negative=[i for i,c in terms.items() if c==-1 and i>0]
    for e in negative:
        for f in negative:
            d=f-e
            if d<=e:
                continue
            expected={2*d:1,d+e:-1,e:-1,0:-1}
            if terms!=expected:
                continue
            # Insert U_d-U_d, distribute, and collect the identical signed words.
            p=(1 << d)+1
            q=(1 << d)-(1 << e)-1
            assert p*q==n
            if q>1:
                returns.append(dict(d=d,e=e,zeroPairPosition=d,
                                    collection='(U_d+U_0)*(U_d-U_e-U_0)',
                                    factors=[str(p),str(q)]))
    return dict(source=str(n),rewriteOrder='higher-first' if higher_first else 'lower-first',rewriteSteps=steps,finalSignedWord=sorted(terms.items()),
                collectedReturns=returns,extractionEvidence='True' if returns else 'None')


if __name__=='__main__':
    reports=[read(n) for n in [221,8051,
         1199215083673205158356661431646321119743288893586546308999201,
         1522605027922533360535618378132637429718068114961380688657908494580122963258952897654000350692006139]]
    write_record(Path(__file__).with_name('godel_symbolic_boundary_collection'), dict(reports=reports))
    print(json.dumps([dict(source=r['source'],rewrites=len(r['rewriteSteps']),remainingTerms=len(r['finalSignedWord']),returns=r['collectedReturns']) for r in reports]))
