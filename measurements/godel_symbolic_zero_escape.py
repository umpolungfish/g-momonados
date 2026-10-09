"""Expose partial collection with a local zero rewrite before cancellation."""
from godel_markdown_records import read_record,write_record
from godel_symbolic_partial_absorption import step
from godel_symbolic_cancel_collect import cancel
from godel_symbolic_boundary_collect import value


def magnitude(t):
    return sum(abs(c) for c in t.values())


def descend(A,B,R,reverse,higher):
    moves=[]
    while True:
        result=step(A,B,R,reverse)
        if result is not None:
            A,B,R,move=result
            moves.append(dict(collection=move))
        R,rewrites=cancel(R,higher)
        if rewrites:
            moves.append(dict(cancellations=rewrites))
        if result is None and not rewrites:
            return A,B,R,moves


def main():
    parents=read_record('godel_symbolic_cancel_partial')['reports']
    seeds=[]
    for n in sorted(set(r['source'] for r in parents),key=int):
        score=min(magnitude(dict(r['correction'])) for r in parents if r['source']==n)
        seen=set()
        for i,r in enumerate(parents):
            if r['source']!=n or magnitude(dict(r['correction']))!=score:
                continue
            key=tuple(tuple(map(tuple,r[k])) for k in ('operandA','operandB','correction'))
            if key in seen:
                continue
            seen.add(key)
            seeds.append((i,r))
    reports=[]
    for index,parent in seeds:
        A,B,R=(dict(parent[k]) for k in ('operandA','operandB','correction'))
        n=int(parent['source']); rounds=[]; checked=0
        while True:
            score=magnitude(R); best=None
            for position,c in sorted(R.items()):
                if abs(c)!=1:
                    continue
                altered=dict(R)
                altered[position]=-c
                altered[position+1]=altered.get(position+1,0)+c
                altered={i:d for i,d in altered.items() if d}
                assert value(altered)==value(R)
                for reverse in (False,True):
                    for higher in (False,True):
                        checked+=1
                        a,b,r,moves=descend(dict(A),dict(B),dict(altered),reverse,higher)
                        assert value(a)*value(b)+value(r)==n
                        final=magnitude(r)
                        if final>=score or (best is not None and final>=best[0]):
                            continue
                        best=(final,a,b,r,dict(polarityRewrite=[position,c],shiftReverse=reverse,
                            cancellationHigherFirst=higher,descent=moves,beforeMagnitude=score,afterMagnitude=final))
            if best is None:
                break
            _,A,B,R,record=best
            rounds.append(record)
        closed=not R and abs(value(A))>1 and abs(value(B))>1
        reports.append(dict(source=str(n),parentState=index,checkedPaths=checked,rounds=rounds,
            operandA=sorted(A.items()),operandB=sorted(B.items()),correction=sorted(R.items()),
            exactReturnEvidence='True',sourceFactorEvidence='True' if closed else 'None'))
    write_record('godel_symbolic_zero_escape',dict(parentRecord='godel_symbolic_cancel_partial',
        seedSelection='All distinct minimum correction-magnitude terminal shapes per source',reports=reports))
    for r in reports:
        print('Source:',r['source'],'parent:',r['parentState'],'paths:',r['checkedPaths'],
            'accepted rounds:',len(r['rounds']),'residual magnitude:',magnitude(dict(r['correction'])),
            'factor evidence:',r['sourceFactorEvidence'])


if __name__=='__main__':
    main()
