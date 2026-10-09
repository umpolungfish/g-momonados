"""Complete a source-derived signed rectangle with an explicit zero pair."""
from godel_markdown_records import read_record, write_record
import json
from pathlib import Path
from godel_symbolic_boundary_collect import value

root=Path(__file__).parent
previous=read_record(root/'godel_symbolic_residual_chains')
row=previous['reports'][2]
terms=dict(row['terminalResidual'])
assert terms=={0:1,35:-1,120:-1,134:1}
a,b,c=134,120,35
rectangleTop=b+c
expanded=[(a,1),(b,-1),(c,-1),(0,1),(rectangleTop,1),(rectangleTop,-1)]
assert value(terms)==sum(sign*(1 << i) for i,sign in expanded)
left={b:1,0:-1}
right={c:1,0:-1}
correction={a:1,rectangleTop:-1}
assert value(terms)==value(left)*value(right)+value(correction)
gap={rectangleTop-a:1,0:-1}
placement={a:1}
assert value(correction)==-value(gap)*value(placement)
report=dict(source=row['source'],residual=sorted(terms.items()),
            insertedZero=[(rectangleTop,1),(rectangleTop,-1)],
            rectangle=dict(left=sorted(left.items()),right=sorted(right.items())),
            correction=sorted(correction.items()),
            correctionCollected=dict(sign=-1,payload=sorted(gap.items()),placement=sorted(placement.items())),
            exactReturnEvidence='True',sourceFactorEvidence='None',
            constraint='The rectangle is only one component. Its correction remains part of the source and cannot be discarded.')
write_record(root/'godel_symbolic_rectangle_completion', report)
print(json.dumps(report))
