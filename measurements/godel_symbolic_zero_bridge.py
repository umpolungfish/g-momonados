"""Connect equal-return signed shapes using only local merge-zero relations."""
from godel_markdown_records import read_record, write_record
import json
from pathlib import Path
from godel_symbolic_boundary_collect import read,value


def bridge(left,right):
    assert value(left)==value(right)
    positions=set(left)|set(right)
    difference={i:left.get(i,0)-right.get(i,0) for i in positions}
    moves=[]
    previous=0
    restored={}
    for i in range(max(positions,default=0)+1):
        coefficient=difference.get(i,0)+previous
        assert coefficient % 2==0
        current=coefficient//2
        if current:
            moves.append(dict(position=i,signedMultiplicity=current,relation='2*U_i-U_(i+1)'))
            restored[i]=restored.get(i,0)+2*current
            restored[i+1]=restored.get(i+1,0)-current
        previous=current
    assert previous==0
    assert {i:c for i,c in restored.items() if c}=={i:c for i,c in difference.items() if c}
    return moves


if __name__=='__main__':
    sources=[221,8051,
       1199215083673205158356661431646321119743288893586546308999201,
       1522605027922533360535618378132637429718068114961380688657908494580122963258952897654000350692006139]
    reports=[]
    for n in sources:
        left=dict(read(n,True)['finalSignedWord'])
        right=dict(read(n,False)['finalSignedWord'])
        reports.append(dict(source=str(n),higherFirst=sorted(left.items()),lowerFirst=sorted(right.items()),
                            bridge=bridge(left,right),exactBridgeEvidence='True',sourceFactorEvidence='None'))
    output=dict(relation='D_i=2*k_i-k_(i-1); k_(-1)=0; every finite zero-return signed word is sum k_i*(2*U_i-U_(i+1))',
                proof='The weighted prefix sum through i is the negative higher-position tail, hence a multiple of 2^(i+1). Its quotient is k_i. The final quotient is zero.',
                scope='Completeness of linear value-preserving unit rewrites, not factor-support selection or arbitrary factor extraction',reports=reports)
    write_record(Path(__file__).with_name('godel_symbolic_zero_bridges'), output)
    print(json.dumps([dict(source=r['source'],localZeroRelations=len(r['bridge'])) for r in reports]))
