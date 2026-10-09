"""Repeated literal collection on retained residuals, with no arithmetic search."""
from godel_markdown_records import read_record, write_record
import json
from pathlib import Path
from godel_symbolic_boundary_collect import value


def next_collection(terms):
    positions=sorted(terms)
    best=None
    for d in sorted({b-a for a in positions for b in positions if b>a}):
        for sign in (1,-1):
            matches=[i for i in positions if terms.get(i+d)==sign*terms[i]]
            for descending in (False,True):
                used=set()
                starts=[]
                for i in sorted(matches,reverse=descending):
                    if i in used or i+d in used:
                        continue
                    used.update((i,i+d))
                    starts.append(i)
                if len(starts)<2 or best and len(starts)<=len(best['payload']):
                    continue
                anchor=min(starts)
                payload={i-anchor:terms[i] for i in starts}
                placement={anchor:1,anchor+d:sign}
                residual={i:c for i,c in terms.items() if i not in used}
                assert value(terms)==value(payload)*value(placement)+value(residual)
                best=dict(payload=payload,placement=placement,residual=residual)
    return best


def chain(source,terms):
    initial=dict(terms)
    products=[]
    while True:
        step=next_collection(terms)
        if step is None:
            break
        products.append(dict(payload=sorted(step['payload'].items()),placement=sorted(step['placement'].items()),
                             beforeTerms=len(terms),afterTerms=len(step['residual'])))
        terms=step['residual']
    assert sum(value(dict(p['payload']))*value(dict(p['placement'])) for p in products)+value(terms)==source
    return dict(source=str(source),sourceSignedWord=sorted(initial.items()),products=products,
                terminalResidual=sorted(terms.items()),exactReturnEvidence='True',sourceFactorEvidence='None')


if __name__=='__main__':
    root=Path(__file__).parent
    previous=read_record(root/'godel_symbolic_translation_collections')
    reports=[chain(int(r['source']),dict(s['sourceSignedWord'])) for r in previous['reports']
             for s in r['shapes']]
    write_record(root/'godel_symbolic_residual_chains', dict(reports=reports))
    print(json.dumps([dict(source=r['source'],products=len(r['products']),terminalTerms=len(r['terminalResidual']),
                           termSequence=[p['beforeTerms'] for p in r['products']]+[len(r['terminalResidual'])]) for r in reports]))
