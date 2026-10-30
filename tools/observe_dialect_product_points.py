#!/usr/bin/env python3
"""Observe exact product and support residuals in a verified terminal family."""
import argparse
import hashlib
import json
import math
import mmap
from pathlib import Path
from verify_dialect_factor import bank_value, numeral, numeral_word, support_polynomial


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project',type=Path)
    parser.add_argument('returned',type=Path)
    parser.add_argument('verification',type=Path)
    parser.add_argument('output',type=Path)
    args=parser.parse_args()
    binding=json.loads(args.verification.read_text())
    digest=hashlib.sha256()
    with args.returned.open('rb') as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b''):digest.update(chunk)
    assert digest.hexdigest()==binding['receipt_sha256'], 'unverified return'
    tables=json.loads((args.project/'dialect_tables.json').read_text())
    n=numeral(tables['source_word'])
    residuals=[];support_zeros=roots=unit_derivatives=proper_arms=unit_quotients=0
    operand_widths={}
    with args.returned.open('rb') as stream,mmap.mmap(stream.fileno(),0,access=mmap.ACCESS_READ) as raw:
        marker=b',"members":[';position=raw.find(marker)
        assert position>=0
        position+=len(marker)
        while position<raw.size()-3:
            next_position=raw.find(b',{"dialect":',position)
            end=next_position if next_position>=0 else raw.rfind(b']}')
            member=json.loads(raw[position:end]);frame=member['product_frame_state']
            p,q=bank_value(frame['anchor_left']),bank_value(frame['anchor_right'])
            unit_quotients+=q==1
            widths=f'{p.bit_length()}:{q.bit_length()}'
            operand_widths[widths]=operand_widths.get(widths,0)+1
            residuals.append(n-p*q)
            value,derivative=support_polynomial(n,p)
            support_zeros+=value==0;roots+=p==2
            unit_derivatives+=math.gcd(derivative,n)==1
            proper_arms+=1<math.gcd(p,n)<n
            if next_position<0:break
            position=next_position+1
    assert len(residuals)==binding['members']
    output={'question':__doc__,'source_bits':n.bit_length(),'members':len(residuals),
        'return_sha256':binding['receipt_sha256'],
        'zero_product_residuals':sum(x==0 for x in residuals),
        'minimum_absolute_product_residual_word':numeral_word(min(map(abs,residuals))),
        'maximum_absolute_product_residual_bits':max(abs(x).bit_length() for x in residuals),
        'support_zeros':support_zeros,'canonical_root_two':roots,
        'unit_support_derivatives':unit_derivatives,'proper_source_arms':proper_arms,
        'unit_quotient_anchors':unit_quotients,'left_right_operand_width_counts':operand_widths}
    args.output.write_text(json.dumps(output,indent=2,ensure_ascii=False)+'\n')
    print(json.dumps(output,indent=2,ensure_ascii=False))


if __name__=='__main__':main()
