#!/usr/bin/env python3
"""Read an exact sibling-dialect source fuse independently of the native probe."""
import argparse
import hashlib
import json
import math
from pathlib import Path
from verify_dialect_factor import numeral, bank_value

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('returned',type=Path)
parser.add_argument('probe',type=Path)
parser.add_argument('output',type=Path)
args=parser.parse_args()
parent=json.loads(args.returned.read_text());probe=json.loads(args.probe.read_text())
assert parent['source_word']==probe['source_word'], 'source changed across sibling fuse'
n=numeral(parent['source_word']);assert n.bit_length()>=200
width=n.bit_length()+10
candidates=[bank_value(member['state'][:width]) for member in parent['members']]
pairs=nonzero=0;found=None
for i,a in enumerate(candidates):
    for j in range(i):
        pairs+=1;difference=abs(a-candidates[j])
        if not difference:continue
        nonzero+=1;divisor=math.gcd(difference,n)
        if 1<divisor<n:
            found=(i,j,divisor,n//divisor);break
    if found:break
assert pairs==probe['pairs'] and nonzero==probe['nonzero_differences'], 'pair traversal differs'
assert bool(found)==probe['closed'], 'native fuse differs from exact divisibility'
if found:
    i,j,p,q=found
    assert (i,j)==(probe['left_member'],probe['right_member'])
    assert numeral(probe['divisor_word'])==p and numeral(probe['quotient_word'])==q
    assert n%p==0 and p*q==n
result={'source_bits':n.bit_length(),'pairs_checked':pairs,'nonzero_differences':nonzero,
        'closed':bool(found),'exact_divisibility_readback':True,
        'probe_sha256':hashlib.sha256(args.probe.read_bytes()).hexdigest()}
if found:
    result['divisor_word']=probe['divisor_word']
    result['quotient_word']=probe['quotient_word']
args.output.write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n')
print(json.dumps(result,indent=2,ensure_ascii=False))
