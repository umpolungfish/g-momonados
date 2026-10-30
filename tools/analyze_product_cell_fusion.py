#!/usr/bin/env python3
"""Compare primitive supports with the joint relation of a retained adder cell."""
import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct


def cells(ops):
    signature = [3, 3, 4, 4, 7, 3, 3, 4, 4, 7, 4, 4, 7]
    for s in range(len(ops)-12):
        block = ops[s:s+13]
        if [row[0] for row in block] != signature:
            continue
        a, b, c = block[0][1], block[1][1], block[6][1]
        if len({a,b,c}) != 3:
            continue
        expected = [(3,a,0),(3,b,0),(4,a,s+1),(4,s,b),(7,s+2,s+3),
                    (3,s+4,0),(3,c,0),(4,s+4,s+6),(4,s+5,c),(7,s+7,s+8),
                    (4,a,b),(4,s+4,c),(7,s+10,s+11)]
        if all(row[:3] == want and row[3] == 0 for row, want in zip(block,expected)):
            yield s, (a,b,c,s+9,s+12)


def row_outputs(ops, start, ports):
    rows = []
    for values in itertools.product([False,True], repeat=3):
        state = dict(zip(ports[:3],values))
        for wire in range(start,start+13):
            op,a,b,_=ops[wire]
            state[wire] = not state[a] if op == 3 else (
                state[a] and state[b] if op == 4 else state[a] or state[b])
        row = tuple(1 if state[wire] else 2 for wire in ports)
        assert bool(row[3]==1) == bool(sum(values)%2)
        assert bool(row[4]==1) == bool(sum(values)>=2)
        rows.append(row)
    return rows


def contract_primitive(ops,start,ports,external):
    domain = {i:3 for i in range(start,start+13)}
    domain.update(zip(ports,external))
    while True:
        old=domain.copy()
        for wire in range(start,start+13):
            op,a,b,_=ops[wire]
            allowed={a:0,wire:0}
            if op!=3:allowed[b]=0
            for av in [False,True]:
                for bv in ([False] if op==3 else [False,True]):
                    out = not av if op==3 else (av and bv if op==4 else av or bv)
                    assignment={a:1 if av else 2,wire:1 if out else 2}
                    if op!=3:assignment[b]=1 if bv else 2
                    if all(domain[i] & v for i,v in assignment.items()):
                        for i,v in assignment.items():allowed[i]|=v
            for i,mask in allowed.items():domain[i]&=mask
            if any(v==0 for v in domain.values()):return None
        if old==domain:return tuple(domain[wire] for wire in ports)


def contract_joint(rows,external):
    surviving=[row for row in rows if all(value & mask for value,mask in zip(row,external))]
    if not surviving:return None
    return tuple(__import__('functools').reduce(int.__or__,column) for column in zip(*surviving))


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('packed_ops',type=Path);p.add_argument('output',type=Path)
    args=p.parse_args();raw=args.packed_ops.read_bytes();ops=list(struct.iter_unpack('<IIII',raw))
    found=list(cells(ops))
    if not found:raise ValueError('no existing native product adder cell found')
    start,ports=found[0];rows=row_outputs(ops,start,ports)
    stronger=[]
    for external in itertools.product([1,2,3],repeat=5):
        primitive=contract_primitive(ops,start,ports,external);joint=contract_joint(rows,external)
        if joint!=primitive:
            assert primitive is not None
            stronger.append({'external':external,'primitive_return':primitive,'joint_return':joint})
    result={'relation_sha256':hashlib.sha256(raw).hexdigest(),'full_adder_cells':len(found),
            'first_cell_start':start,'ports_a_b_c_sum_carry':ports,'joint_rows':rows,
            'all_external_support_patterns':3**5,'strictly_stronger_patterns':len(stronger),
            'contradictions_exposed_only_by_joint_cell':sum(x['joint_return'] is None for x in stronger),
            'examples':stronger[:8],
            'carrier':'joint native T/F cell assignments; 1=T, 2=F, 3=both alternatives'}
    args.output.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))


if __name__=='__main__':main()
