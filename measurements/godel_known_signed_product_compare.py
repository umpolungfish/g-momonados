"""Compare user-supplied product operands via signed cancellation and zero bridges."""
from godel_markdown_records import read_record, write_record
import json
from pathlib import Path
from godel_symbolic_boundary_collect import read,value
from godel_symbolic_zero_bridge import bridge

root=Path(__file__).parent
triple=read_record(root/'godel_known_semiprime_comparison')['triple']
n,p,q=(int(triple[k]) for k in ('N','P','Q'))
assert p*q==n
source=dict(read(n,True)['finalSignedWord'])
reports=[]
for left_order in (True,False):
    left=dict(read(p,left_order)['finalSignedWord'])
    for right_order in (True,False):
        right=dict(read(q,right_order)['finalSignedWord'])
        raw={}
        contributions={}
        for i,a in left.items():
            for j,b in right.items():
                raw[i+j]=raw.get(i+j,0)+a*b
                slot=contributions.setdefault(i+j,dict(positive=0,negative=0))
                slot['positive' if a*b>0 else 'negative']+=abs(a*b)
        raw={i:c for i,c in raw.items() if c}
        assert value(raw)==n
        zero_bridge=bridge(raw,source)
        cancelled=sum(min(c['positive'],c['negative']) for c in contributions.values())
        reports.append(dict(leftOrder='higher-first' if left_order else 'lower-first',
                            rightOrder='higher-first' if right_order else 'lower-first',
                            leftSignedWord=sorted(left.items()),rightSignedWord=sorted(right.items()),
                            rawSignedProduct=sorted(raw.items()),perPositionContributions=sorted(contributions.items()),
                            oppositePairsCancelled=cancelled,
                            bridge=zero_bridge,positiveZeroMultiplicities=sum(m['signedMultiplicity']>0 for m in zero_bridge),
                            negativeZeroMultiplicities=sum(m['signedMultiplicity']<0 for m in zero_bridge),
                            maximumAbsoluteZeroMultiplicity=max((abs(m['signedMultiplicity']) for m in zero_bridge),default=0)))
output=dict(triple=triple,operandKnowledge='User-supplied comparison factors, not extracted operands',
            sourceSignedWord=sorted(source.items()),reports=reports,
            warningAboutScope='Signed contributions are not FOUR evidence values. Nonnegative-transfer bounds apply only to nonnegative expanded supports, not to these signed states.',
            exactProductEvidence='True',sourceOnlyExtractionEvidence='None')
write_record(root/'godel_known_signed_product_comparison', output)
print(json.dumps([dict(leftTerms=len(r['leftSignedWord']),rightTerms=len(r['rightSignedWord']),
       rawTerms=len(r['rawSignedProduct']),cancelledPairs=r['oppositePairsCancelled'],
       positiveMoves=r['positiveZeroMultiplicities'],negativeMoves=r['negativeZeroMultiplicities'],
       maxMultiplicity=r['maximumAbsoluteZeroMultiplicity']) for r in reports]))
