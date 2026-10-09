"""Frame product sums at each separation, with exact reversed-frame closure."""
from godel_markdown_records import read_record, write_record
import json
import math
from pathlib import Path


def probe(n):
    counts = dict(framings=0, productSums=0, unit=0, wholeSource=0, proper=0, reverseClosures=0)
    hits=[]
    for width in range(1,n.bit_length()+1):
        B=1 << width
        for shift in range(width):
            tail=n >> shift
            frames=[]
            remainder=tail
            while remainder:
                frames.append(remainder & (B-1))
                remainder >>= width
            m=len(frames)
            correlations=[sum(frames[i]*frames[i+k] for i in range(m-k)) for k in range(m)]
            reverse=sum(a*B**(m-1-i) for i,a in enumerate(frames))
            reconstructed=correlations[0]*B**(m-1)
            for k in range(1,m):
                reconstructed+=correlations[k]*(B**(m-1-k)+B**(m-1+k))
            assert reconstructed==tail*reverse
            counts['framings']+=1
            counts['reverseClosures']+=1
            for k,value in enumerate(correlations):
                g=math.gcd(value,n)
                counts['productSums']+=1
                category='unit' if g==1 else 'wholeSource' if g==n else 'proper'
                counts[category]+=1
                if category=='proper':
                    assert g*(n//g)==n
                    hits.append(dict(width=width,shift=shift,separation=k,result=str(value),
                                     factor=str(g),cofactor=str(n//g)))
    return dict(source=str(n),counts=counts,hits=hits,factorEvidence='True' if hits else 'None')


if __name__=='__main__':
    reports=[probe(n) for n in [8051,
         1199215083673205158356661431646321119743288893586546308999201,
         1522605027922533360535618378132637429718068114961380688657908494580122963258952897654000350692006139]]
    output=dict(operation='C_k=sum a_i*a_(i+k) over every valid frame pair',
                closure='tail*reverseFrames= C_0*B^(m-1)+sum C_k*(B^(m-1-k)+B^(m-1+k)) for k>0',
                shiftPrefixRetained='original source is prefix+2^s*tail; gcd uses original source',reports=reports)
    write_record(Path(__file__).with_name('godel_frame_cross_product_relationships'), output)
    print(json.dumps([dict(source=x['source'],counts=x['counts'],hits=x['hits'][:6]) for x in reports]))
