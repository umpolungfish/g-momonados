#!/usr/bin/env python3
"""Check the prepared source-parity pair in every open resident return."""
import argparse
import hashlib
import json
import mmap
from pathlib import Path
from verify_dialect_factor import bank_value, numeral

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('project',type=Path)
parser.add_argument('returned',type=Path)
parser.add_argument('output',type=Path)
args=parser.parse_args()
tables=json.loads((args.project/'dialect_tables.json').read_text())
prepared=json.loads((args.project/'prepared.json').read_text())
width=tables['source_bits']+10
expected=prepared['state'][3*width:5*width]
even,odd=bank_value(expected[:width]),bank_value(expected[width:])
woven=sum(((even>>i)&1)<<(2*i) | ((odd>>i)&1)<<(2*i+1) for i in range(width))
assert woven==numeral(tables['source_word']), 'prepared pending pair does not recover source'
count=0
with args.returned.open('rb') as stream,mmap.mmap(stream.fileno(),0,access=mmap.ACCESS_READ) as raw:
    boundary=raw.find(b',"members":[')
    header=json.loads(raw[:boundary]+b'}')
    assert not header['closed'] and header['source_word']==tables['source_word']
    position=boundary
    while True:
        position=raw.find(b'"state":[',position)
        if position<0:break
        position+=len(b'"state":')
        end=raw.find(b']',position)+1
        state=json.loads(raw[position:end])
        assert len(state)==14*width and state[3*width:5*width]==expected, 'pending source pair changed'
        count+=1;position=end
    assert count==header['held']==3*len(tables['tables'])
with args.returned.open('rb') as stream:digest=hashlib.file_digest(stream,'sha256').hexdigest()
result={'question':'Do all open alternatives retain the prepared source parity pair?',
        'source_bits':tables['source_bits'],'alternatives_checked':count,
        'source_partition_roundtrip':True,'pending_pair_banks_preserved':True,'receipt_sha256':digest}
args.output.write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
