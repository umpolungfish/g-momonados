"""Alternate exact local residual cancellation and partial-copy collection."""
from godel_markdown_records import read_record, write_record
from godel_symbolic_cancel_collect import cancel
from godel_symbolic_partial_absorption import step
from godel_symbolic_boundary_collect import value


def main():
    parents=read_record('godel_symbolic_partial_absorption')['reports']
    reports=[]
    for index,parent in enumerate(parents):
        n=int(parent['source'])
        for higher in (False,True):
            A,B,R=(dict(parent[k]) for k in ('operandA','operandB','correction'))
            rounds=[]
            while True:
                before=sum(abs(c) for c in R.values())
                R,rewrites=cancel(R,higher)
                result=step(A,B,R,parent['shiftOrder']=='higher-first')
                move=None
                if result is not None:
                    A,B,R,move=result
                after=sum(abs(c) for c in R.values())
                assert value(A)*value(B)+value(R)==n
                if not rewrites and move is None:
                    break
                assert after<before
                rounds.append(dict(cancellations=rewrites,partialCollection=move,
                    beforeMagnitude=before,afterMagnitude=after))
            closed=not R and abs(value(A))>1 and abs(value(B))>1
            reports.append(dict(source=str(n),parentState=index,
                cancellationOrder='higher-first' if higher else 'lower-first',rounds=rounds,
                operandA=sorted(A.items()),operandB=sorted(B.items()),correction=sorted(R.items()),
                exactReturnEvidence='True',sourceFactorEvidence='True' if closed else 'None'))
    write_record('godel_symbolic_cancel_partial',dict(parentRecord='godel_symbolic_partial_absorption',reports=reports))
    print('States:',len(reports),'changed:',sum(bool(r['rounds']) for r in reports),
          'closed:',sum(r['sourceFactorEvidence']=='True' for r in reports))
    for n in sorted(set(r['source'] for r in reports),key=int):
        rows=[r for r in reports if r['source']==n]
        print('Source:',n,'minimum correction terms:',min(len(r['correction']) for r in rows),
            'minimum magnitude:',min(sum(abs(c) for i,c in r['correction']) for r in rows),
            'maximum rounds:',max(len(r['rounds']) for r in rows))


if __name__=='__main__':
    main()
