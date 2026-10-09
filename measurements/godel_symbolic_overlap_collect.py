"""Keep overlapping signed copies and their exact source correction."""
from godel_markdown_records import write_record
from godel_symbolic_boundary_collect import read,value
from godel_symbolic_rewrite_absorption import merge


def expand(a,b):
    t={}
    for i,c in a.items():
        for j,d in b.items():
            t[i+j]=t.get(i+j,0)+c*d
    return {i:c for i,c in t.items() if c}


def collect(n):
    source=dict(read(n,True)['finalSignedWord'])
    positions=sorted(source)
    states=[]
    for offset in sorted({b-a for a in positions for b in positions if b>a}):
        for sign in (1,-1):
            matches=[i for i in positions if source.get(i+offset)==sign*source[i]]
            if len(matches)<2:
                continue
            overlap=set(matches)&{i+offset for i in matches}
            if not overlap:
                continue
            anchor=min(matches)
            A={i-anchor:source[i] for i in matches}
            B={anchor:1,anchor+offset:sign}
            raw=expand(A,B)
            correction={i:source.get(i,0)-raw.get(i,0) for i in set(source)|set(raw)}
            correction={i:c for i,c in correction.items() if c}
            normalized,moves=merge(correction)
            assert value(A)*value(B)+value(normalized)==n
            states.append(dict(offset=offset,orientation=sign,payload=sorted(A.items()),
                                placement=sorted(B.items()),overlapPositions=sorted(overlap),
                                rawProduct=sorted(raw.items()),rawCorrection=sorted(correction.items()),
                                correction=sorted(normalized.items()),correctionMerges=moves,
                                exactDecompositionEvidence='True',sourceFactorEvidence='True' if not normalized and abs(value(A))>1 and abs(value(B))>1 else 'None'))
    return dict(source=str(n),sourceSignedWord=sorted(source.items()),states=states)


if __name__=='__main__':
    reports=[collect(n) for n in [21,8051,
       1199215083673205158356661431646321119743288893586546308999201,
       1522605027922533360535618378132637429718068114961380688657908494580122963258952897654000350692006139]]
    write_record('godel_symbolic_overlap_collections',dict(rule='Keep overlapping copies; expand their signed contributions and retain source-product correction',reports=reports))
    for r in reports:
        print('Source',r['source'],'overlap states',len(r['states']),
              'largest payload',max((len(s['payload']) for s in r['states']),default=0),
              'closed',sum(s['sourceFactorEvidence']=='True' for s in r['states']))
