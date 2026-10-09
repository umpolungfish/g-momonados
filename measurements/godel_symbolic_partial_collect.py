"""Retain alternative cancellation shapes and exact partial signed products."""
from godel_markdown_records import read_record, write_record
import json
from pathlib import Path
from godel_symbolic_boundary_collect import read,value


def shifted(t,offset):
    return {i+offset:c for i,c in t.items()}


def collect(n):
    shapes=[]
    for higher in (True,False):
        base=read(n,higher)
        terms=dict(base['finalSignedWord'])
        collections=[]
        # A matching signed pair is an identical payload translated twice.
        # Match source positions and signs, not numerical divisor values.
        pairs={}
        positions=sorted(terms)
        for j,lo in enumerate(positions):
            for hi in positions[j+1:]:
                if abs(terms[lo])!=1 or abs(terms[hi])!=1:
                    continue
                payload=((0,terms[lo]),(hi-lo,terms[hi]))
                pairs.setdefault(payload,[]).append(lo)
        for payload,starts in pairs.items():
            for j,first in enumerate(starts):
                for second in starts[j+1:]:
                    p=dict(payload)
                    left,right=shifted(p,first),shifted(p,second)
                    if set(left)&set(right):
                        continue
                    residual=dict(terms)
                    for i,c in list(left.items())+list(right.items()):
                        residual[i]-=c
                    residual={i:c for i,c in residual.items() if c}
                    placement={first:1,second:1}
                    assert value(p)*value(placement)+value(residual)==n
                    collections.append(dict(payload=sorted(p.items()),placement=sorted(placement.items()),
                                            residualShape=dict(source='containing shape sourceSignedWord',
                                                               subtractSignedTerms=sorted(list(left.items())+list(right.items())),
                                                               remainingTermCount=len(residual)),
                                            partialProductEvidence='True',
                                            sourceFactorEvidence='True' if not residual and value(p)>1 and value(placement)>1 else 'None'))
        shapes.append(dict(order=base['rewriteOrder'],sourceSignedWord=base['finalSignedWord'],
                           rewrites=base['rewriteSteps'],collections=collections))
    return dict(source=str(n),shapes=shapes,
                interpretation='Different symbolic shapes retain the same source. Partial collection preserves its signed residual; it does not certify a source factor.')


if __name__=='__main__':
    reports=[collect(n) for n in [221,8051,
        1199215083673205158356661431646321119743288893586546308999201,
        1522605027922533360535618378132637429718068114961380688657908494580122963258952897654000350692006139]]
    write_record(Path(__file__).with_name('godel_symbolic_partial_collections'), dict(reports=reports))
    print(json.dumps([dict(source=r['source'],shapes=[dict(order=s['order'],terms=len(s['sourceSignedWord']),collections=len(s['collections'])) for s in r['shapes']]) for r in reports]))
