"""Absorb literal payload copies across a product/residual boundary."""
from godel_markdown_records import read_record,write_record
from godel_symbolic_boundary_collect import value


def product(a,b):
    t={}
    for i,c in a.items():
        for j,d in b.items():
            t[i+j]=t.get(i+j,0)+c*d
    return {i:c for i,c in t.items() if c}


def absorb(payload,placement,residual):
    placement,residual=dict(placement),dict(residual)
    moves=[]
    anchor=min(payload)
    while True:
        found=None
        for position in sorted(residual):
            shift=position-anchor
            if shift<0:
                continue
            for sign in (1,-1):
                if all(residual.get(i+shift)==sign*c for i,c in payload.items()):
                    found=(shift,sign)
                    break
            if found:
                break
        if found is None:
            break
        shift,sign=found
        for i in payload:
            del residual[i+shift]
        placement[shift]=placement.get(shift,0)+sign
        placement={i:c for i,c in placement.items() if c}
        moves.append(dict(shift=shift,sign=sign,remainingTerms=len(residual)))
    return placement,residual,moves


if __name__ == "__main__":
    reports=[]
    controls=[(35,{0:1,2:1},{3:1},{0:-1,2:-1}),
              (221,{0:1,4:1},{4:1},{0:-1,1:-1,4:-1,5:-1})]
    for n,A,B,R in controls:
        assert value(A)*value(B)+value(R)==n
        newB,newR,moves=absorb(A,B,R)
        assert not newR and value(A)*value(newB)==n
        reports.append(dict(source=str(n),control='Source-derived signed collection',payload=sorted(A.items()),
                            initialPlacement=sorted(B.items()),initialResidual=sorted(R.items()),
                            placement=sorted(newB.items()),residual=sorted(newR.items()),moves=moves,factorEvidence='True'))
    chains=read_record('godel_symbolic_residual_chains')
    for row in chains['reports']:
        source=dict(row['sourceSignedWord'])
        n=int(row['source'])
        for index,p in enumerate(row['products']):
            for swap in (False,True):
                A,B=dict(p['payload']),dict(p['placement'])
                if swap:
                    A,B=B,A
                initialEmptyCells=min(A)
                A={i-initialEmptyCells:c for i,c in A.items()}
                B={i+initialEmptyCells:c for i,c in B.items()}
                expanded=product(A,B)
                R={i:source.get(i,0)-expanded.get(i,0) for i in set(source)|set(expanded)}
                R={i:c for i,c in R.items() if c}
                newB,newR,moves=absorb(A,B,R)
                assert value(A)*value(newB)+value(newR)==n
                reports.append(dict(source=row['source'],productIndex=index,swap=swap,payload=sorted(A.items()),
                                    initialEmptyCellsTransferred=initialEmptyCells,
                                    initialPlacement=sorted(B.items()),initialResidual=sorted(R.items()),
                                    placement=sorted(newB.items()),residual=sorted(newR.items()),moves=moves,
                                    factorEvidence='True' if not newR and abs(value(A))>1 and abs(value(newB))>1 else 'None'))
    write_record('godel_symbolic_residual_absorption',dict(identity='A*B+(sign*U_s*A+R)=A*(B+sign*U_s)+R',reports=reports))
    print('Source controls closed:',sum(r['factorEvidence']=='True' for r in reports[:2]))
    print('Large-source states checked:',len(reports)-2)
    print('Large-source states with absorption:',sum(bool(r['moves']) for r in reports[2:]))
    print('Large-source whole returns:',sum(r['factorEvidence']=='True' for r in reports[2:]))
