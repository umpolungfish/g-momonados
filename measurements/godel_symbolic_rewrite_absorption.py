"""Expose latent residual copies by one local polarity rewrite and merges."""
from godel_markdown_records import read_record,write_record
from godel_symbolic_residual_absorption import absorb
from godel_symbolic_boundary_collect import value


def merge(t):
    t=dict(t)
    moves=[]
    while True:
        indices=[i for i,c in t.items() if abs(c)>=2]
        if not indices:
            break
        i=min(indices)
        sign=1 if t[i]>0 else -1
        t[i]-=2*sign
        t[i+1]=t.get(i+1,0)+sign
        t={j:c for j,c in t.items() if c}
        moves.append([i,sign])
    return t,moves


if __name__=='__main__':
    prior=read_record('godel_symbolic_residual_absorption')
    checked=0
    successes=[]
    for index,row in enumerate(prior['reports']):
        if 'control' in row:
            continue
        A,B,R=(dict(row[k]) for k in ('payload','placement','residual'))
        for position,c in sorted(R.items()):
            if c not in (1,-1):
                continue
            checked+=1
            altered=dict(R)
            altered[position]=-c
            altered[position+1]=altered.get(position+1,0)+c
            altered={i:v for i,v in altered.items() if v}
            assert value(altered)==value(R)
            normalized,merges=merge(altered)
            newB,newR,moves=absorb(A,B,normalized)
            assert value(A)*value(newB)+value(newR)==int(row['source'])
            if not moves:
                continue
            successes.append(dict(parentState=index,source=row['source'],payload=row['payload'],
                                   polarityRewrite=[position,c],unitMerges=merges,absorptions=moves,
                                   updatedPlacement=sorted(newB.items()),remainingResidual=sorted(newR.items()),
                                   exactReturnEvidence='True',sourceFactorEvidence='None' if newR else 'True'))
    write_record('godel_symbolic_rewrite_absorption',dict(localRewrite='sign*U_i=sign*U_(i+1)-sign*U_i',
                 checkedResidualShapes=checked,statesWithNewAbsorption=successes))
    print('Equivalent residual shapes checked:',checked)
    print('Shapes exposing new absorption:',len(successes))
    print('Whole-source closures:',sum(s['sourceFactorEvidence']=='True' for s in successes))
