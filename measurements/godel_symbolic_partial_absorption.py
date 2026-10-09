"""Collect partial translated payload copies under strict signed-word descent."""
from godel_markdown_records import read_record, write_record
from godel_symbolic_boundary_collect import value


def step(A,B,R,reverse):
    before=sum(abs(c) for c in R.values())
    best=None
    for side,payload in enumerate((A,B)):
        if not payload:
            continue
        shifts={j-i for j in R for i in payload if j>=i}
        for shift in sorted(shifts,reverse=reverse):
            for sign in (1,-1):
                delta=sum(abs(R.get(i+shift,0)-sign*c)-abs(R.get(i+shift,0)) for i,c in payload.items())
                if delta>=0 or (best is not None and delta>=best[0]):
                    continue
                best=(delta,side,shift,sign)
    if best is None:
        return None
    delta,side,shift,sign=best
    payload=A if side==0 else B
    residual=dict(R)
    for i,c in payload.items():
        residual[i+shift]=residual.get(i+shift,0)-sign*c
        if not residual[i+shift]:
            del residual[i+shift]
    updated=dict(B if side==0 else A)
    updated[shift]=updated.get(shift,0)+sign
    if not updated[shift]:
        del updated[shift]
    a,b=(A,updated) if side==0 else (updated,B)
    after=sum(abs(c) for c in residual.values())
    assert after==before+delta and after<before
    return a,b,residual,dict(payloadSide=side,shift=shift,sign=sign,
        beforeMagnitude=before,afterMagnitude=after,correction=sorted(residual.items()))


def main():
    prior=read_record('godel_symbolic_overlap_collections')
    reports=[]
    for row in prior['reports']:
        n=int(row['source'])
        for index,parent in enumerate(row['states']):
            for reverse in (False,True):
                A,B,R=(dict(parent[k]) for k in ('payload','placement','rawCorrection'))
                moves=[]
                while True:
                    result=step(A,B,R,reverse)
                    if result is None:
                        break
                    A,B,R,move=result
                    assert value(A)*value(B)+value(R)==n
                    moves.append(move)
                closed=not R and abs(value(A))>1 and abs(value(B))>1
                reports.append(dict(source=str(n),parentState=index,shiftOrder='higher-first' if reverse else 'lower-first',
                    moves=moves,operandA=sorted(A.items()),operandB=sorted(B.items()),correction=sorted(R.items()),
                    exactReturnEvidence='True',sourceFactorEvidence='True' if closed else 'None'))
    write_record('godel_symbolic_partial_absorption',dict(selection='Greatest strict decrease of correction coefficient absolute sum; shifts arise from aligned occupied signed cells',reports=reports))
    print('States:',len(reports),'changed:',sum(bool(r['moves']) for r in reports),
          'closed:',sum(r['sourceFactorEvidence']=='True' for r in reports))
    for n in sorted(set(r['source'] for r in reports),key=int):
        rows=[r for r in reports if r['source']==n]
        print('Source:',n,'minimum residual terms:',min(len(r['correction']) for r in rows),
            'minimum residual magnitude:',min(sum(abs(c) for i,c in r['correction']) for r in rows),
            'maximum moves:',max(len(r['moves']) for r in rows))


if __name__=='__main__':
    main()
