"""Collect maximal disjoint signed payload copies at source-derived translations."""
from godel_markdown_records import read_record, write_record
import json
from pathlib import Path
from godel_symbolic_boundary_collect import read,value


def collect(n):
    shapes=[]
    for higher in (True,False):
        base=read(n,higher)
        terms=dict(base['finalSignedWord'])
        positions=sorted(terms)
        offsets=sorted({b-a for a in positions for b in positions if b>a})
        collections=[]
        for offset in offsets:
            for sign in (1,-1):
                matching=[i for i in positions if terms.get(i+offset)==sign*terms[i]]
                for direction in ('ascending','descending'):
                    used=set()
                    starts=[]
                    for i in sorted(matching,reverse=direction=='descending'):
                        if i in used or i+offset in used:
                            continue
                        used.update((i,i+offset))
                        starts.append(i)
                    if len(starts)<3:
                        continue
                    anchor=min(starts)
                    payload={i-anchor:terms[i] for i in starts}
                    placement={anchor:1,anchor+offset:sign}
                    residual={i:c for i,c in terms.items() if i not in used}
                    assert value(payload)*value(placement)+value(residual)==n
                    collections.append(dict(offset=offset,orientation=sign,selectionOrder=direction,
                                            payload=sorted(payload.items()),placement=sorted(placement.items()),
                                            residual=sorted(residual.items()),consumedTerms=len(used),
                                            exactDecompositionEvidence='True',sourceFactorEvidence='None'))
        shapes.append(dict(order=base['rewriteOrder'],sourceSignedWord=base['finalSignedWord'],collections=collections))
    return dict(source=str(n),shapes=shapes)


if __name__=='__main__':
    reports=[collect(n) for n in [8051,
       1199215083673205158356661431646321119743288893586546308999201,
       1522605027922533360535618378132637429718068114961380688657908494580122963258952897654000350692006139]]
    write_record(Path(__file__).with_name('godel_symbolic_translation_collections'), dict(reports=reports))
    print(json.dumps([dict(source=r['source'],shapes=[dict(order=s['order'],collections=len(s['collections']),
          largestPayloadTerms=max((len(c['payload']) for c in s['collections']),default=0),
          smallestResidualTerms=min((len(c['residual']) for c in s['collections']),default=len(s['sourceSignedWord']))) for s in r['shapes']]) for r in reports]))
