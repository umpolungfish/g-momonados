"""Check correlated local-zero transport through the supplied signed product."""
from godel_markdown_records import read_record, write_record
import json
from pathlib import Path
from godel_symbolic_zero_bridge import bridge


def convolution(a,b):
    result={}
    for i,x in a.items():
        for j,y in b.items():
            result[i+j]=result.get(i+j,0)+x*y
    return {i:c for i,c in result.items() if c}


def difference(a,b):
    return {i:a.get(i,0)-b.get(i,0) for i in set(a)|set(b) if a.get(i,0)!=b.get(i,0)}


def coefficient_map(moves):
    return {m['position']:m['signedMultiplicity'] for m in moves}


root=Path(__file__).parent
comparison=read_record(root/'godel_known_signed_product_comparison')
hh,hl,lh,ll=comparison['reports']
pH,pL=dict(hh['leftSignedWord']),dict(lh['leftSignedWord'])
qH,qL=dict(hh['rightSignedWord']),dict(hl['rightSignedWord'])
pMoves=coefficient_map(bridge(pH,pL))
qMoves=coefficient_map(bridge(qH,qL))
K=[coefficient_map(r['bridge']) for r in (hh,hl,lh,ll)]
rightTransport=difference(K[0],K[1])
leftTransport=difference(K[0],K[2])
assert rightTransport==convolution(pH,qMoves)
assert leftTransport==convolution(qH,pMoves)
mixed=difference(difference(K[0],K[1]),difference(K[2],K[3]))
pair=convolution(pMoves,qMoves)
expected={i:2*pair.get(i,0)-pair.get(i-1,0) for i in set(pair)|{j+1 for j in pair}}
expected={i:c for i,c in expected.items() if c}
assert mixed==expected
report=dict(source=comparison['triple']['N'],operandKnowledge=comparison['operandKnowledge'],
            localRelation='Z_i=2*U_i-U_(i+1); Z_i*U_j=Z_(i+j)',
            leftOperandMoves=sorted(pMoves.items()),rightOperandMoves=sorted(qMoves.items()),
            rightMoveProductTransport=sorted(rightTransport.items()),leftMoveProductTransport=sorted(leftTransport.items()),
            jointInteraction=sorted(mixed.items()),
            exactTransportEvidence='True',sourceOnlyFactorEvidence='None',
            scope='A translated zero pattern is not independently a factor certificate; these comparisons require coherent supplied operand products.')
write_record(root/'godel_symbolic_product_zero_transport', report)
print(json.dumps(dict(leftOperandMoves=len(pMoves),rightOperandMoves=len(qMoves),
                     leftTransportTerms=len(leftTransport),rightTransportTerms=len(rightTransport),
                     jointInteractionTerms=len(mixed))))
