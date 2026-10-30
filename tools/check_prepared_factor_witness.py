#!/usr/bin/env python3
"""Check both known verification-only orientations after membrane production."""
from array import array
import hashlib
import json
from pathlib import Path
import struct
from verify_dialect_factor import numeral


def check(directory,reference):
    # The producer has already terminated. This instrument receives factors;
    # the source-only compiler and executable never receive this reference.
    report=json.loads((directory/'return.json').read_text())
    witness=json.loads(reference.read_text());p,q=int(witness['p']),int(witness['q'])
    n=numeral(report['source_word'])
    if p*q!=n or not 1<p<n or not 1<q<n:raise ValueError('verification factors differ from source')
    raw=(directory/'relation.ops').read_bytes();domains=(directory/'register.cells').read_bytes()
    topology=(directory/'correlated.topology').read_bytes();state=(directory/'correlated.cells').read_bytes()
    nf,np,nt=struct.unpack_from('<QQQ',topology);at=24;templates=[]
    for _ in range(nt):
        width,count=topology[at:at+2];at+=2
        templates.append([tuple(topology[at+i*width:at+(i+1)*width]) for i in range(count)])
        at+=width*count
    frame_start=at;orientations=[]
    for left,right in ((p,q),(q,p)):
        values=bytearray()
        for wire,(op,a,b,required) in enumerate(struct.iter_unpack('<IIII',raw)):
            if op==0:value=False
            elif op==5:value=True
            elif op==8:value=bool(((left if a%2==0 else right)>>(a//2))&1)
            elif op==3:value=not values[a]
            elif op==4:value=bool(values[a] and values[b])
            elif op==7:value=bool(values[a] or values[b])
            else:raise ValueError('unknown arithmetic operator')
            if required and not value:raise ValueError('reference pair fails the product constraint')
            if not domains[wire]&(1 if value else 2):raise ValueError('prepared register excluded a true factor orientation')
            values.append(value)
        at=frame_start;row_ids=bytearray()
        for frame in range(nf):
            gate,t=struct.unpack_from('<II',topology,at);at+=8;width=len(templates[t][0])
            ports=struct.unpack_from('<'+'I'*width,topology,at);at+=4*width
            row=tuple(1 if values[wire] else 2 for wire in ports)
            try:row_id=templates[t].index(row)
            except ValueError:raise ValueError('native relation excluded an exact arithmetic assignment')
            if not state[frame]&(1<<row_id):raise ValueError('prepared joint rows excluded a true factor orientation')
            row_ids.append(row_id)
        for edge in range(np):
            a,b,_=struct.unpack_from('<IIQ',topology,at+16*edge)
            mask=struct.unpack_from('<Q',state,nf+8*edge)[0]
            if not mask&(1<<(8*row_ids[a]+row_ids[b])):
                raise ValueError('prepared pair fibre excluded a true factor orientation')
        orientations.append({'orientation':'p,q' if left==p else 'q,p',
                             'source_wires_checked':len(values),'native_frames_checked':nf,
                             'correlated_pairs_checked':np})
    result={'source_sha256':hashlib.sha256((directory/'source.imasm').read_bytes()).hexdigest(),
            'reference_sha256':hashlib.sha256(reference.read_bytes()).hexdigest(),
            'scope':'post-production reference check of prepared state only',
            'ordered_orientations':orientations,'exact_reference_product':True}
    return result
