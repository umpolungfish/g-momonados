"""Rewrite payloads while transporting their zero difference into correction."""
from godel_markdown_records import read_record, write_record
from godel_symbolic_cancel_collect import cancel
from godel_symbolic_residual_absorption import absorb, product
from godel_symbolic_rewrite_absorption import merge
from godel_symbolic_boundary_collect import value


def main():
    prior = read_record('godel_symbolic_overlap_collections')
    checked = 0
    reports = []
    for row in prior['reports']:
        source = dict(row['sourceSignedWord'])
        n = int(row['source'])
        for index,state in enumerate(row['states']):
            for swap in (False,True):
                original,B = dict(state['payload']),dict(state['placement'])
                if swap:
                    original,B = B,original
                variants = []
                for higher in (False,True):
                    A,moves = cancel(original,higher)
                    variants.append((['cancel',higher,moves],A))
                for i,c in sorted(original.items()):
                    if abs(c)!=1:
                        continue
                    A=dict(original)
                    A[i]-=2*c
                    A[i+1]=A.get(i+1,0)+c
                    A,moves=merge(A)
                    variants.append((['polarity',i,c,moves],A))
                seen={tuple(sorted(original.items()))}
                for rewrite,A in variants:
                    key=tuple(sorted(A.items()))
                    if key in seen:
                        continue
                    seen.add(key)
                    assert value(A)==value(original)
                    anchor=min(A)
                    payload={i-anchor:c for i,c in A.items()}
                    placement={i+anchor:c for i,c in B.items()}
                    expanded=product(payload,placement)
                    R={i:source.get(i,0)-expanded.get(i,0) for i in set(source)|set(expanded)}
                    R={i:c for i,c in R.items() if c}
                    for order in ('literal','lower','higher'):
                        checked+=1
                        residual=dict(R)
                        residualMoves=[]
                        if order!='literal':
                            residual,residualMoves=cancel(residual,order=='higher')
                        newB,newR,copies=absorb(payload,placement,residual)
                        assert value(payload)*value(newB)+value(newR)==n
                        if not copies:
                            continue
                        closed=not newR and abs(value(payload))>1 and abs(value(newB))>1
                        reports.append(dict(source=str(n),parentState=index,swap=swap,
                            payloadRewrite=rewrite,payload=sorted(payload.items()),
                            initialPlacement=sorted(placement.items()),residualOrder=order,
                            residualRewrites=residualMoves,absorptions=copies,
                            placement=sorted(newB.items()),correction=sorted(newR.items()),
                            exactReturnEvidence='True',sourceFactorEvidence='True' if closed else 'None'))
    write_record('godel_symbolic_payload_rewrite',dict(checkedShapes=checked,
        retention='Absorbing states retained; nonabsorbing states counted, no factor exclusion',reports=reports))
    print('Checked:',checked,'absorbing states:',len(reports),'closures:',sum(r['sourceFactorEvidence']=='True' for r in reports))
    for n in sorted(set(r['source'] for r in reports),key=int):
        rows=[r for r in reports if r['source']==n]
        print('Source:',n,'absorptions:',len(rows),'minimum correction terms:',min(len(r['correction']) for r in rows))


if __name__=='__main__':
    main()
