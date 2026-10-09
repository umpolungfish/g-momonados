"""Explicit unit-word cancellation, splitting, and merging; no numerical search."""
from godel_markdown_records import read_record, write_record
import json
from pathlib import Path


def value(terms):
    return sum(count*(1 << position) for position,count in terms.items())


def pretty(terms):
    # U_i is descriptive notation for a word occupied only at position i.
    return [{'position':i,'signedMultiplicity':c} for i,c in sorted(terms.items()) if c]


left={6:1,4:1,2:1,0:-1}
right={7:1,5:-1,0:1}
expanded=[]
terms={}
for i,a in left.items():
    for j,b in right.items():
        expanded.append(dict(leftPosition=i,rightPosition=j,resultPosition=i+j,sign=a*b))
        terms[i+j]=terms.get(i+j,0)+a*b
assert value(terms)==8051
steps=[dict(operation='expand positioned product and cancel equal-position opposite signs',terms=pretty(terms))]
assert terms[11]==0 and terms[9]==0

# Two negative occupied contributions merge into one at the next position.
assert terms[7]==-2
terms[7]=0
terms[8]=terms.get(8,0)-1
assert value(terms)==8051
steps.append(dict(operation='merge -U_7 - U_7 into -U_8',terms=pretty(terms)))

# Rewrite U_6+U_5+U_4 as U_7-U_4 using explicit split and cancellation.
assert terms[4]==1
# Separate the original U_4 into 2*U_4-U_4, then merge successively.
terms[4]-=2
terms[5]+=1
assert value(terms)==8051
steps.append(dict(operation='U_4 = (U_4+U_4)-U_4; merge the positive pair to U_5',terms=pretty(terms)))
assert terms[5]==2
terms[5]=0
terms[6]+=1
assert value(terms)==8051
steps.append(dict(operation='merge U_5+U_5 into U_6',terms=pretty(terms)))
assert terms[6]==2
terms[6]=0
terms[7]+=1
assert value(terms)==8051
steps.append(dict(operation='merge U_6+U_6 into U_7',terms=pretty(terms)))
expected={13:1,8:-1,7:1,4:-1,2:1,0:-1}
assert {i:c for i,c in terms.items() if c}==expected

report=dict(source='8051',unitNotation='U_i means the encoded word occupied only at position i; this is descriptive notation, not a new Grammar opcode',
            operands=['83','97'],operandOrigin='Previously verified square-sum return; comparison operands, not a new extraction claim',
            leftSignedWord=pretty(left),rightSignedWord=pretty(right),expandedProduct=expanded,
            cancelledPositions=[9,11],steps=steps,
            sourceRunBoundaryWord=pretty(expected),
            sourceRuns=[[0,2],[4,7],[8,13]],
            payloadLSBFirst='⊥⊥⊤⊤⊥⊥⊥⊤⊥⊥⊥⊥⊥',
            verified='Every rewrite preserves the source; final signed boundaries restore the exact source payload')
write_record(Path(__file__).with_name('godel_symbolic_cancellation_8051'), report)
print(json.dumps(dict(cancelledPositions=report['cancelledPositions'],steps=len(steps),finalTerms=pretty(terms))))
