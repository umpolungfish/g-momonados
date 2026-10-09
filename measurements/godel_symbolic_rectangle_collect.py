"""Collect literal translated signed payloads after a single unit rewrite."""
from godel_markdown_records import read_record, write_record
import json
from pathlib import Path
from godel_symbolic_boundary_collect import read, value


def rectangle(terms):
    if len(terms)!=4 or terms.get(0) not in (-1,1):
        return []
    positive_positions=sorted(i for i in terms if i>0)
    a,b,top=positive_positions
    if top!=a+b or terms[top]!=1:
        return []
    alpha,beta=terms[b],terms[a]
    if alpha not in (-1,1) or beta not in (-1,1) or terms[0]!=alpha*beta:
        return []
    p,q=(1 << a)+alpha,(1 << b)+beta
    if p<=1 or q<=1:
        return []
    assert p*q==value(terms)
    return [dict(positions=[a,b],signs=[alpha,beta],factors=[str(p),str(q)])]


def probe(n):
    base=read(n)
    terms=dict(base['finalSignedWord'])
    states=[dict(rewrite='retain signed word',terms=sorted(terms.items()),returns=rectangle(terms))]
    for i,c in sorted(terms.items()):
        if c not in (-1,1):
            continue
        t=dict(terms)
        t[i]=-c
        t[i+1]=t.get(i+1,0)+c
        t={j:v for j,v in t.items() if v}
        assert value(t)==n
        states.append(dict(rewrite=f'{c:+d}U_{i} -> {c:+d}U_{i+1} {-c:+d}U_{i}',
                           terms=sorted(t.items()),returns=rectangle(t)))
    return dict(source=str(n),boundaryRewrites=base['rewriteSteps'],states=states,
                factorEvidence='True' if any(s['returns'] for s in states) else 'None')


if __name__=='__main__':
    reports=[probe(n) for n in [35,221,8051,
        1199215083673205158356661431646321119743288893586546308999201,
        1522605027922533360535618378132637429718068114961380688657908494580122963258952897654000350692006139]]
    write_record(Path(__file__).with_name('godel_symbolic_rectangle_collection'), dict(reports=reports))
    print(json.dumps([dict(source=r['source'],states=len(r['states']),returns=[s for s in r['states'] if s['returns']]) for r in reports]))
