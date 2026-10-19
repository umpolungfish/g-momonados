#!/usr/bin/env python3
"""Independently audit native arithmetic returns and correlated topology."""
from array import array
import hashlib
import gzip
import json
from pathlib import Path
import struct


def audit(directory, report, ops, cells):
    binding=json.loads((directory/'binding.json').read_text())
    for name,expected in binding['correlated_artifacts'].items():
        if hashlib.sha256((directory/name).read_bytes()).hexdigest()!=expected:
            raise ValueError('changed correlated artifact: '+name)
    native=json.loads((directory/'native_arithmetic.json').read_text())
    from compile_native_arithmetic_bridge import compile_bridge
    paths=list(native['rules'])
    rebuilt=json.loads(json.dumps(compile_bridge(Path(paths[0]),Path(paths[1]))))
    if native!=rebuilt:
        raise ValueError('native arithmetic return differs from its bound rule execution')
    from relational_dialect import NativeRules
    rules=NativeRules(paths[0],paths[1])
    saved=report['resident_dialect']
    if 'native_types' in saved:
        steps=report['bridge_phase'];start=native['cycle_start'];length=native['cycle_length']
        replay=start+(steps-start)%length if steps>start else steps
        types={kind:gate['stages'][0]['before'] for kind,gate in native['gates'].items()}
        for phase in range(replay):
            dialect,operation=native['route'][phase%len(native['route'])]
            for kind,gate in native['gates'].items():
                types[kind]=[list(rules.compose(dialect,operation,a,b)[0])
                             for a,b in zip(types[kind],gate['stages'][0]['right'])]
        if saved['native_steps']!=steps or saved['native_types']!=types:
            raise ValueError('returned native tuple bank differs from its executed rule continuation')
    raw=(directory/'correlated.topology').read_bytes()
    nf,np,nt=struct.unpack_from('<QQQ',raw);at=24;templates=[]
    for _ in range(nt):
        width,count=raw[at:at+2];at+=2
        if not 1<=width<=5 or not 1<=count<=8:raise ValueError('invalid correlated row template')
        rows=[tuple(raw[at+i*width:at+(i+1)*width]) for i in range(count)]
        at+=width*count;templates.append(rows)
    offsets=array('Q');template_ids=array('I');gates=array('I')
    kinds={}
    for frame in range(nf):
        gate,template=struct.unpack_from('<II',raw,at)
        width=len(templates[template][0]);offsets.append(at+8);at+=8+4*width
        ports=struct.unpack_from('<'+'I'*width,raw,offsets[-1])
        if gate<len(ops):
            op,a,b,_=ops[gate]
            expected=(a,gate) if op==3 else (a,b,gate)
            if op not in (3,4,7) or ports!=expected:raise ValueError('native frame lost source ports')
            rows=[]
            for av in (0,1):
                for bv in ((0,) if op==3 else (0,1)):
                    if op!=3 and a==b and av!=bv:continue
                    ov=1-av if op==3 else av&bv if op==4 else av|bv
                    row=(av,ov) if op==3 else (av,bv,ov)
                    rows.append(tuple(1 if v else 2 for v in row))
        else:
            if gate-len(ops)>=len(cells) or ports!=cells[gate-len(ops)]:
                raise ValueError('native carry frame lost source ports')
            rows=[]
            for i in range(8):
                bits=[(i>>j)&1 for j in range(3)];n=sum(bits)
                rows.append(tuple(1 if v else 2 for v in bits+[n%2,int(n>=2)]))
        if rows!=templates[template]:raise ValueError('native arithmetic row differs from exact predicate')
        template_ids.append(template);gates.append(gate)
    expected_gates=array('I',(i for i,row in enumerate(ops) if row[0] in (3,4,7)))
    expected_gates.extend(range(len(ops),len(ops)+len(cells)))
    if gates!=expected_gates:raise ValueError('native arithmetic frame omitted or reordered')
    pair_start=at
    if pair_start+16*np!=len(raw):raise ValueError('correlated topology has trailing or missing bytes')
    prepared=(directory/'correlated.cells').read_bytes()
    saved=report['resident_dialect']
    current=(gzip.decompress(bytes.fromhex(saved['state_gzip_hex'])) if 'state_gzip_hex' in saved
             else bytes.fromhex(saved['state_hex']))
    if len(prepared)!=nf+8*np or len(current)!=len(prepared):raise ValueError('correlated state dimensions changed')
    if any(a&~b for a,b in zip(current,prepared)):raise ValueError('returned correlations exceeded prepared alternatives')
    for i,t in enumerate(template_ids):
        if not current[i] or prepared[i]>>len(templates[t]):raise ValueError('empty or invalid native frame mask')
    classes={};previous=None
    for edge in range(np):
        left,right,admitted=struct.unpack_from('<IIQ',raw,pair_start+edge*16)
        if not 0<=left<right<nf or (previous is not None and (left,right)<=previous):
            raise ValueError('invalid or repeated correlated edge')
        previous=(left,right)
        a,b=templates[template_ids[left]],templates[template_ids[right]]
        ap=struct.unpack_from('<'+'I'*len(a[0]),raw,offsets[left])
        bp=struct.unpack_from('<'+'I'*len(b[0]),raw,offsets[right])
        shared=tuple((i,j) for i,x in enumerate(ap) for j,y in enumerate(bp) if x==y)
        key=(template_ids[left],template_ids[right],shared)
        if key not in classes:
            if not shared:raise ValueError('unbound correlated frames')
            classes[key]=sum(1<<(8*i+j) for i,ar in enumerate(a) for j,br in enumerate(b)
                             if all(ar[x]==br[y] for x,y in shared))
        if admitted!=classes[key]:raise ValueError('pair admission changed a shared arithmetic assignment')
        pm=struct.unpack_from('<Q',prepared,nf+8*edge)[0]
        cm=struct.unpack_from('<Q',current,nf+8*edge)[0]
        if pm&~admitted or not cm:raise ValueError('invalid correlated pair mask')
    # Read a factor-bit requirement from the actual N-only source circuit.
    # The output truth observation is supplied by that circuit's close condition.
    deduction=None
    for wire,(op,a,b,required) in enumerate(ops):
        if op!=4 or not required or ops[a][0]!=5 or ops[b][0]!=8:continue
        images={}
        for dialect in (0,22):
            groups={}
            for av in (0,1):
                for bv in (0,1):
                    image,_=rules.compose(dialect,'join',rules.digit_types[av],rules.digit_types[bv])
                    groups.setdefault(image,[]).append((av,bv))
            observed={image for image,fibres in groups.items() if any(av&bv for av,bv in fibres)}
            compatible=[(av,bv) for image,fibres in groups.items() if image in observed
                        for av,bv in fibres if av==1]
            images[str(dialect)]={'surviving_inputs':compatible,
                                 'factor_bit_support':sum(1<<bit for bit in set(bv for _,bv in compatible))}
        if images['22']['factor_bit_support']!=2 or images['0']['factor_bit_support']!=3:
            raise ValueError('native rule sensitivity did not reproduce')
        deduction={'source_gate':wire,'factor_wire':b,'factor_variable':ops[b][1],
                   'source_requirement':'AND output true; left operand is the source circuit truth constant',
                   'native_observation_pullbacks':images}
        break
    if deduction is None:raise ValueError('actual source factor boundary not found')
    return prepared,current,{'native_frames_checked':nf,'correlated_pairs_checked':np,
                             'pair_binding_classes_checked':len(classes),
                             'rule_dependent_source_deduction':deduction}
