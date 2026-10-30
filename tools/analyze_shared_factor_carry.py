#!/usr/bin/env python3
"""Find a source-bound carry seam whose product terms share an unknown factor bit."""
import argparse
import hashlib
import itertools
import json
from pathlib import Path
import struct
from analyze_product_cell_fusion import cells


def fused_relation(ops, pair, native_rows):
    outputs = {wire for ports in pair for wire in ports[3:]}
    terms = {}
    leaves = set()
    for ports in pair:
        for wire in ports[:3]:
            if wire in outputs:
                continue
            op,a,b,_ = ops[wire]
            if op == 4 and ops[a][0] == ops[b][0] == 8:
                terms[wire] = (a,b)
                leaves.update([a,b])
            else:
                leaves.add(wire)
    leaves = sorted(leaves)
    scope = sorted(set(leaves) | set(terms) | outputs)
    transitions = {tuple(row[:3]): tuple(row[3:]) for row in native_rows}
    rows = []
    for values in itertools.product([1,2], repeat=len(leaves)):
        state = dict(zip(leaves,values))
        for wire,(a,b) in terms.items():
            state[wire] = 1 if state[a] == state[b] == 1 else 2
        for ports in pair:
            state[ports[3]],state[ports[4]] = transitions[tuple(state[w] for w in ports[:3])]
        rows.append(tuple(state[w] for w in scope))
    components = [(list(ports),native_rows) for ports in pair]
    and_rows = [(a,b,1 if a == b == 1 else 2) for a,b in itertools.product([1,2],repeat=2)]
    components += [([a,b,wire],and_rows) for wire,(a,b) in terms.items()]
    return scope,rows,components,leaves


def projection(rows, domains):
    result = [0]*len(domains)
    for row in rows:
        if all(v & d for v,d in zip(row,domains)):
            for i,v in enumerate(row):result[i] |= v
    return result


def local_closure(components, domain):
    domain=domain.copy()
    while True:
        old=domain.copy()
        for ports,rows in components:
            allowed=projection(rows,[domain[w] for w in ports])
            if not all(allowed):return None
            for wire,value in zip(ports,allowed):domain[wire] &= value
        if domain==old:return domain


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('returned_directory',type=Path)
    p.add_argument('output',type=Path)
    args=p.parse_args();raw=(args.returned_directory/'relation.ops').read_bytes()
    ops=list(struct.iter_unpack('<IIII',raw))
    returned_raw=(args.returned_directory/'return.json').read_bytes()
    returned=json.loads(returned_raw);domains=returned['register']
    native_rows=list(struct.iter_unpack('BBBBB',(args.returned_directory/'cell.rows').read_bytes()))
    cell_list=[ports for _,ports in cells(ops)]
    readers={}
    for index,ports in enumerate(cell_list):
        for wire in ports[:3]:readers.setdefault(wire,[]).append(index)
    def factor_inputs(ports):
        result=set()
        for wire in ports[:3]:
            op,a,b,_=ops[wire]
            if op==4 and ops[a][0]==ops[b][0]==8:result.update([a,b])
        return result
    factor_sets=[factor_inputs(ports) for ports in cell_list]
    examined=0;eligible=0;witness=None
    for first,ports in enumerate(cell_list):
        for second in readers.get(ports[4],[]):
            shared=factor_sets[first] & factor_sets[second]
            shared={wire for wire in shared if domains[wire]==3}
            if not shared:continue
            eligible+=1
            pair=[ports,cell_list[second]]
            scope,rows,components,leaves=fused_relation(ops,pair,native_rows)
            initial={wire:domains[wire] for wire in scope}
            local=local_closure(components,initial);examined+=1
            if local is None:continue
            allowed=projection(rows,[local[w] for w in scope])
            if allowed==[local[w] for w in scope]:continue
            changes=[]
            for wire,value in zip(scope,allowed):
                if value!=local[wire]:
                    op,variable,_,_=ops[wire]
                    change={'wire':wire,'separate_support':local[wire],'fused_support':value}
                    if op==8:change.update(lane='p' if variable%2==0 else 'q',bit=variable//2)
                    changes.append(change)
            witness={'cells':pair,'shared_unknown_factor_wires':sorted(shared),
                     'scope':scope,'source_domains':[initial[w] for w in scope],
                     'separate_fixed_point':[local[w] for w in scope],
                     'fused_projection':allowed,'changes':changes,
                     'joint_rows':len(rows),'external_leaves':leaves}
            break
        if witness:break
    result={'relation_sha256':hashlib.sha256(raw).hexdigest(),
            'return_sha256':hashlib.sha256(returned_raw).hexdigest(),
            'source_word':returned['source_word'],'eligible_carry_pairs_before_stop':eligible,
            'pairs_examined_before_stop':examined,'first_stronger_fusion':witness}
    args.output.write_text(json.dumps(result,indent=2)+'\n')
    summary={k:v for k,v in result.items() if k not in ['source_word','first_stronger_fusion']}
    summary['witness']=witness
    print(json.dumps(summary))


if __name__=='__main__':main()
