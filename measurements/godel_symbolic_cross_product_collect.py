"""Collect filled-block payloads across a retained source sum of products."""
from godel_markdown_records import read_record, write_record
import json
from pathlib import Path
from godel_symbolic_boundary_collect import value

root=Path(__file__).parent
chains=read_record(root/'godel_symbolic_residual_chains')
row=chains['reports'][2]
completion=read_record(root/'godel_symbolic_rectangle_completion')
products=[dict(left=dict(p['payload']),right=dict(p['placement'])) for p in row['products']]
assert row['source']==completion['source'] and row['terminalResidual']==completion['residual']
products.append(dict(left=dict(completion['rectangle']['left']),right=dict(completion['rectangle']['right'])))
correction=completion['correctionCollected']
products.append(dict(left={i:correction['sign']*c for i,c in correction['placement']},right=dict(correction['payload'])))
n=int(row['source'])
assert sum(value(p['left'])*value(p['right']) for p in products)==n


def tile_difference(terms,d):
    if len(terms)!=2:
        return None
    positions=sorted(terms)
    low,high=positions
    if terms[low]!=-terms[high] or abs(terms[low])!=1:
        return None
    length=high-low
    if length % d:
        return None
    # U_high-U_low is a contiguous filled interval, tiled into d-cell blocks.
    quotient={low+i:terms[high] for i in range(0,length,d)}
    assert value(terms)==((1 << d)-1)*value(quotient)
    return quotient


reports=[]
for d in (3,5,7):
    grouped=[]
    untouched=[]
    for index,p in enumerate(products):
        side='left'
        quotient=tile_difference(p[side],d)
        if quotient is None:
            side='right'
            quotient=tile_difference(p[side],d)
        if quotient is None:
            untouched.append(index)
            continue
        other=p['right' if side=='left' else 'left']
        grouped.append(dict(productIndex=index,commonSide=side,
                            tiledPlacement=sorted(quotient.items()),otherPayload=sorted(other.items())))
    R={d:1,0:-1}
    inner=sum(value(dict(p['tiledPlacement']))*value(dict(p['otherPayload'])) for p in grouped)
    rest=sum(value(products[i]['left'])*value(products[i]['right']) for i in untouched)
    assert value(R)*inner+rest==n
    reports.append(dict(commonPayload=sorted(R.items()),groupedProducts=grouped,
                        remainingProductIndices=untouched,exactCollectionEvidence='True',sourceFactorEvidence='None'))
output=dict(source=str(n),originalProducts=[dict(left=sorted(p['left'].items()),right=sorted(p['right'].items())) for p in products],
            alternativeCollections=reports)
write_record(root/'godel_symbolic_cross_product_collections', output)
print(json.dumps([dict(span=d,grouped=[p['productIndex'] for p in r['groupedProducts']],remaining=r['remainingProductIndices']) for d,r in zip((3,5,7),reports)]))
