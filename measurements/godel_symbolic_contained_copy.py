"""Subtract signed payload copies without discarding unused multiplicities."""
from godel_markdown_records import read_record, write_record
from godel_symbolic_boundary_collect import value


def absorb_contained(A,B,R):
    B,R=dict(B),dict(R)
    moves=[]
    if not A:
        return B,R,moves
    anchor=min(A)
    while True:
        found=None
        for position in sorted(R):
            shift=position-anchor
            if shift<0:
                continue
            for sign in (1,-1):
                if all(R.get(i+shift,0)*sign*c>0 and
                       abs(R.get(i+shift,0))>=abs(c) for i,c in A.items()):
                    found=(shift,sign)
                    break
            if found is not None:
                break
        if found is None:
            return B,R,moves
        shift,sign=found
        before=sum(abs(c) for c in R.values())
        for i,c in A.items():
            R[i+shift]-=sign*c
            if not R[i+shift]:
                del R[i+shift]
        B[shift]=B.get(shift,0)+sign
        if not B[shift]:
            del B[shift]
        after=sum(abs(c) for c in R.values())
        assert before-after==sum(abs(c) for c in A.values())
        moves.append(dict(shift=shift,sign=sign,beforeMagnitude=before,afterMagnitude=after))


def collect(A,B,R,first):
    A,B,R=dict(A),dict(B),dict(R)
    initial=value(A)*value(B)+value(R)
    steps=[]
    while True:
        changed=False
        for side in ((0,1) if first==0 else (1,0)):
            updated,residual,moves=absorb_contained(A if side==0 else B,B if side==0 else A,R)
            if not moves:
                continue
            if side==0:
                B=updated
            else:
                A=updated
            R=residual
            changed=True
            assert value(A)*value(B)+value(R)==initial
            steps.append(dict(payloadSide=side,moves=moves,operandA=sorted(A.items()),
                              operandB=sorted(B.items()),remainingCorrection=sorted(R.items())))
        if not changed:
            return A,B,R,steps


def main():
    a,b,r,steps=collect({0:1,1:1},{2:1},{0:1,1:2,2:1},0)
    assert not r and value(a)*value(b)==21
    controls=[dict(source='21',scope='Constructed overlapping-copy control',steps=steps,
                   factors=[str(value(a)),str(value(b))])]
    prior=read_record('godel_symbolic_overlap_collections')
    reports=[]
    for row in prior['reports']:
        for index,state in enumerate(row['states']):
            for shape in ('rawCorrection','correction'):
                for first in (0,1):
                    a,b,r,steps=collect(dict(state['payload']),dict(state['placement']),dict(state[shape]),first)
                    assert value(a)*value(b)+value(r)==int(row['source'])
                    closed=not r and abs(value(a))>1 and abs(value(b))>1
                    reports.append(dict(source=row['source'],parentState=index,correctionShape=shape,
                        firstPayload=first,steps=steps,operandA=sorted(a.items()),operandB=sorted(b.items()),
                        correction=sorted(r.items()),exactReturnEvidence='True',
                        sourceFactorEvidence='True' if closed else 'None'))
    write_record('godel_symbolic_contained_copy',dict(controls=controls,reports=reports))
    print('Constructed control closed: 3*7=21')
    print('Source-derived states:',len(reports),'absorbing:',sum(bool(r['steps']) for r in reports),
          'closed:',sum(r['sourceFactorEvidence']=='True' for r in reports))
    for n in sorted(set(r['source'] for r in reports),key=int):
        rows=[r for r in reports if r['source']==n]
        print('Source:',n,'absorbing:',sum(bool(r['steps']) for r in rows),
              'minimum correction terms:',min(len(r['correction']) for r in rows))


if __name__=='__main__':
    main()
