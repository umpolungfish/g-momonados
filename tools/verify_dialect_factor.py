#!/usr/bin/env python3
"""Independently read returned source fibres, residuals, banks and factors."""
import argparse
import copy
import hashlib
import json
import math
import mmap
import gzip
from pathlib import Path


def numeral(word):
    cells = [c for c in word if c in '⊤⊥']
    expected = '⊢' + ''.join('≻⋈∈'+c+'∋' for c in cells) + '⊙⊡⊣'
    assert word == expected, 'noncanonical numeral'
    assert not cells or cells[-1] == '⊥', 'untrimmed numeral'
    return sum((c == '⊥') << i for i,c in enumerate(cells))


def numeral_word(value):
    """Return canonical IMASM digits directly from binary integer parity."""
    assert value >= 0
    return '⊢' + ''.join('≻⋈∈' + ('⊥' if (value >> i) & 1 else '⊤') + '∋'
                         for i in range(value.bit_length())) + '⊙⊡⊣'


def bank_value(cells):
    assert all(v in (8,9) for v in cells), 'invalid parity bank'
    return sum((v == 9) << i for i,v in enumerate(cells))


def image_of(value,n,table,rotation,count,radix,keep_fibres):
    image=0
    for j in range(count):
        a=(n>>(4*((j+rotation)%count)))&15;b=(value>>(4*j))&15
        address=table[16*a+b][0]
        image=(image*radix+(address*256+16*a+b if keep_fibres else address))%n
    return image


def support_polynomial(n,x):
    value=derivative=0
    for i in range(n.bit_length()-1,-1,-1):
        derivative=(derivative*x+value)%n
        value=(value*x+((n>>i)&1))%n
    return value,derivative


def support_pair(n,x,y):
    px=py=divided=0
    for i in range(n.bit_length()-1,-1,-1):
        divided=(y*divided+px)%n
        bit=(n>>i)&1;px=(x*px+bit)%n;py=(y*py+bit)%n
    assert (px-py-(x-y)*divided)%n==0
    return py,divided


def verified_parent(data,tables,prepared):
    encoded=data.get('parent_return_gzip_hex','')
    if not encoded:return None,0
    try:raw=gzip.decompress(bytes.fromhex(encoded));parent=json.loads(raw)
    except (ValueError,OSError) as error:raise AssertionError('invalid retained parent payload') from error
    assert hashlib.sha256(raw).hexdigest()==data['parent_return_sha256'], 'retained parent binding changed'
    earlier,count=verified_parent(parent,tables,prepared)
    report=verify(parent,tables,prepared,prior=earlier)
    assert not parent['closed'], 'cannot resume an already closed source'
    return parent,count+report['transitions_checked']


def verify(data, tables, prepared, partial=False, prior=None):
    assert data['source_word'] == tables['source_word'], 'source mismatch'
    terminal_only=data.get('terminal_only_source_readout',False)
    if terminal_only:
        assert data['transport_source_probe_calls']==0, 'source readout occurred during transport'
        if 'terminal_unchanged_members' in data:
            expected_readbacks=data['terminal_factor_readout']['member']+1 if data['closed'] else data['held']
            assert data['terminal_unchanged_members']==data['terminal_sic_readbacks']==expected_readbacks, 'terminal readout changed frozen family'
            if data.get('terminal_only_frame_recovery'):assert data['terminal_frame_readbacks']==expected_readbacks
            assert 0<=data['trigger_to_transport_end_seconds']<=data['trigger_to_readout_seconds']
            assert abs(data['trigger_to_readout_seconds']-data['trigger_to_transport_end_seconds']-data['terminal_readout_seconds'])<0.002
    n = numeral(data['source_word'])
    width = tables['source_bits']+10
    assert n.bit_length() == tables['source_bits'] >= 200
    count = (tables['source_bits']+3)//4
    radix = tables['crystal_radix'] * (256 if data.get('keep_fibres') else 1)
    distinct=set(); transitions=0; changed=0; closed=[];frames=units=frame_changes=point_updates=support_calls=support_reseeds=0;promotion_steps=promotion_changes=terminal_scanned=secant_calls=secant_changed=0
    previous={(m['dialect'],m['operation']):m for m in prior['members']} if prior else {}
    assert 3*len(tables['tables']) == data['held']
    if not partial:assert len(data['members']) == data['held']
    seen=set()
    for member in data['members']:
        u,op=member['dialect'],member['operation']
        assert (u,op) not in seen, 'duplicate dialect alternative'
        seen.add((u,op))
        baseline=previous.get((u,op),prepared)
        pending=[bank_value(word) for word in baseline.get('deferred_source_arms',[])]
        original=[bank_value(baseline['state'][i*width:(i+1)*width]) for i in range(14)]
        assert len(member['state']) == 14*width
        banks=[bank_value(member['state'][i*width:(i+1)*width]) for i in range(14)]
        e=original[0]; exponent=original[5]; seed=original[6]
        last_trivial=original[7];last_residual=original[8];singular_projection=original[10]
        restarts=baseline['receipt'][5];projecting=baseline['receipt'][6];singularities=baseline['receipt'][7]
        conflicts=baseline.get('conflicts',0);gates=baseline.get('gate_admitted',0);time=baseline.get('t_admitted',0);local_changes=0
        expected_complement=original[2];expected_power=original[9]
        initial_frame=baseline.get('product_frame_state',{})
        frame_initialized=initial_frame.get('initialized',False)
        matrix=[bank_value(word) for word in initial_frame['matrix']] if frame_initialized else [1,0,0,1]
        frame_left=bank_value(initial_frame['left']) if frame_initialized else original[3]
        frame_right=bank_value(initial_frame['right']) if frame_initialized else original[4]
        anchor_left=bank_value(initial_frame['anchor_left']) if frame_initialized and 'anchor_left' in initial_frame else original[3]
        anchor_right=bank_value(initial_frame['anchor_right']) if frame_initialized and 'anchor_right' in initial_frame else original[4]
        for index,step in enumerate(member['ancestry']):
            target=(u+baseline['receipt'][1]-prepared['receipt'][1]+index)%len(tables['tables']) if data.get('successive_dialect_hops') else u
            if 'ruleset' in step:assert step['ruleset']==target, 'dialect continuation changed'
            table=tables['tables'][target]['operations'][op]
            if 'before' in step:assert numeral(step['before']) == e, 'broken preimage chain'
            if 'residual' in step:assert numeral(step['residual']) == e*(e-1)%n, 'incorrect banked residual'
            expected_rotation=(baseline['receipt'][1]+index)%count
            assert step['rotation'] == expected_rotation, 'relative phase lost'
            assert len(step['labels'])==len(step['addresses'])==count
            rebuilt_n=rebuilt_e=image=0
            for j,(label,address) in enumerate(zip(step['labels'],step['addresses'])):
                a,b=label>>4,label&15
                cell=table[16*a+b]
                assert cell[0] == address, 'dialect algebra address changed'
                rebuilt_n |= a << (4*((j+step['rotation'])%count))
                rebuilt_e |= b << (4*j)
                conflicts |= cell[1];gates += cell[2];time += cell[3]
                coordinate=address*256+label if data.get('keep_fibres') else address
                image=(image*radix+coordinate)%n
            assert rebuilt_n==n and rebuilt_e==e, 'source/preimage recovery failed'
            unpromoted_image=image
            if data.get('promotion_return'):
                axis=(baseline['receipt'][1]+index)%12
                assert step['promotion_axis']==axis, 'promotion axis continuation changed'
                cards=[len(values) for values in tables['axis_values']]
                stride=math.prod(cards[axis+1:])
                target_index=tables['promotion_target_indices'][axis]
                promoted=[address-(address//stride%cards[axis])*stride+target_index*stride for address in step['addresses']]
                assert step['promoted_addresses']==promoted, 'promotion certificate changed'
                promotion_steps+=1;promotion_changes+=sum(a!=b for a,b in zip(step['addresses'],promoted))
                image=0
                for address,label in zip(promoted,step['labels']):
                    image=(image*radix+address*256+label)%n
                for original_address,returned in zip(step['addresses'],promoted):
                    source_index=original_address//stride%cards[axis]
                    assert returned-target_index*stride+source_index*stride==original_address, 'promotion return lost source coordinate'
            local_changes += image!=e;e=image;transitions+=1;exponent+=1
            expected_power=(2*e-1)%n;expected_complement=n+1-e
            def proper_gcd(value):
                divisor=math.gcd(value,n)
                return 1<divisor<n
            frame_closed=False
            point_closed=False
            support_closed=False
            if data.get('product_frame'):
                p,q=anchor_left,anchor_right;residual=abs(n-p*q)
                a=image
                old_left,old_right=(frame_left,frame_right) if data.get('cumulative_product_frame') else (p,q)
                b=image_of(old_left,n,table,step['rotation'],count,radix,data.get('keep_fibres'))
                c=image_of(old_right,n,table,step['rotation'],count,radix,data.get('keep_fibres'))
                d=image_of(residual,n,table,step['rotation'],count,radix,data.get('keep_fibres'))
                left=(a*old_left+b*old_right)%n;right=(c*old_left+d*old_right)%n
                if data.get('cumulative_product_frame'):
                    x,y,z,w=matrix
                    matrix=[(a*x+b*z)%n,(a*y+b*w)%n,(c*x+d*z)%n,(c*y+d*w)%n]
                else:matrix=[a,b,c,d]
                ma,mb,mc,md=matrix;det=(ma*md-mb*mc)%n
                assert [bank_value(step[k]) for k in ('frame_left','frame_right','frame_determinant')]==[left,right,det], 'product frame image changed'
                assert left==(ma*p+mb*q)%n and right==(mc*p+md*q)%n, 'cumulative product frame lost its source'
                back_p=(md*left-mb*right)%n;back_q=(ma*right-mc*left)%n
                assert back_p==det*p%n and back_q==det*q%n
                assert (back_p*back_q-det*det*p*q)%n==0, 'transported product relation failed'
                unit=math.gcd(det,n)==1
                if data.get('terminal_only_frame_recovery'):
                    assert not step['frame_checked'] and not step['frame_unit'], 'frame recovery ran during transport'
                else:assert step['frame_checked'] and step['frame_unit']==unit
                assert step['frame_changed']==(left!=old_left or right!=old_right)
                if unit:
                    inv=pow(det,-1,n)
                    assert back_p*inv%n==p and back_q*inv%n==q, 'product frame inverse failed'
                factors=[math.gcd(value,n) for value in (det,left,right,back_p,back_q)]
                factor=next((value for value in factors if 1<value<n),None)
                if terminal_only:pending.extend([det,left,right,(md*left-mb*right)%n,(ma*right-mc*left)%n])
                frame_closed=factor is not None and not terminal_only
                assert step['frame_closed']==frame_closed, 'product frame source fuse differs'
                if frame_closed:assert banks[3]==factor and index==len(member['ancestry'])-1
                frames+=1;units+=unit;frame_changes+=step['frame_changed']
                frame_left,frame_right=left,right;frame_initialized=True
                if data.get('cofactor_feedback') and not frame_closed:
                    boundary=right if op==2 else left;parity=1 if op==1 else 0
                    proposal=sum(((boundary>>(2*i+parity))&1)<<i for i in range(width) if 2*i+parity<width)
                    if data.get('support_newton'):
                        support_point=p
                        if data.get('promotion_return'):
                            shift=(e-unpromoted_image)%n;support_point=(p+shift)%n
                            assert [bank_value(step[k]) for k in ('promotion_shift','promotion_point')]==[shift,support_point], 'promotion operand transport changed'
                            assert (support_point-shift)%n==p
                        value,derivative=support_polynomial(n,support_point)
                        assert step['support_evaluated'], 'source support evaluation lost'
                        assert [bank_value(step[k]) for k in ('support_value','support_derivative')]==[value,derivative], 'source support polynomial/derivative changed'
                        tangent_proposal=proposal
                        if math.gcd(derivative,n)==1 and value:
                            tangent_point=(support_point-value*pow(derivative,-1,n))%n
                            if 1<tangent_point<n:tangent_proposal=tangent_point
                        denominator=derivative
                        if data.get('support_secant'):
                            other,denominator=support_pair(n,support_point,p)
                            assert [bank_value(step[k]) for k in ('support_other_value','support_divided')]==[other,denominator], 'two-point support coefficient changed'
                            assert (value-other-(support_point-p)*denominator)%n==0
                        support_unit=math.gcd(denominator,n)==1
                        assert step['support_unit']==support_unit, 'source support coefficient inverse differs'
                        probes=[math.gcd(x,n) for x in (derivative,value,(support_point-2)%n)]
                        factor=next((x for x in probes if 1<x<n),None)
                        if terminal_only:pending.extend([derivative,value,(support_point-2)%n])
                        if terminal_only and data.get('support_secant'):pending.extend([denominator,other,shift])
                        support_closed=factor is not None and not terminal_only
                        assert step['support_closed']==support_closed, 'source support fuse differs'
                        if support_closed:assert banks[3]==factor and index==len(member['ancestry'])-1
                        reseeded=False
                        if not support_closed:
                            if support_unit and value:
                                next_point=(support_point-value*pow(denominator,-1,n))%n
                                if 1<next_point<n:proposal=next_point
                                else:reseeded=True
                            else:reseeded=True
                        if data.get('support_secant'):
                            secant_calls+=1;secant_changed+=proposal!=tangent_proposal
                        assert step['support_reseeded']==reseeded, 'source support continuation differs'
                        support_calls+=1;support_reseeds+=reseeded
                    update=1<proposal<n and not support_closed
                    assert step['point_updated']==update, 'product point update lost'
                    if update:
                        quotient,remainder=divmod(n,proposal)
                        assert proposal*quotient+remainder==n
                        anchor_left,anchor_right=proposal,quotient
                        frame_left=(ma*proposal+mb*quotient)%n
                        frame_right=(mc*proposal+md*quotient)%n
                        if unit:
                            inv=pow(det,-1,n)
                            assert (md*frame_left-mb*frame_right)*inv%n==proposal
                            assert (ma*frame_right-mc*frame_left)*inv%n==quotient
                        if terminal_only:pending.append(proposal)
                        point_closed=proper_gcd(proposal) and not terminal_only
                        if point_closed:assert banks[3]==math.gcd(proposal,n)
                        point_updates+=1
                    assert step['point_closed']==point_closed, 'product point source fuse differs'
            if terminal_only:pending.extend([e,expected_complement])
            if data.get('compose_idempotent') and not frame_closed and not point_closed and not support_closed and (terminal_only or not (proper_gcd(e) or proper_gcd(expected_complement))):
                if e in (0,1):
                    last_trivial=e;last_residual=0;restarts+=1;seed+=1
                    e=seed;exponent=2;expected_power=(2*e-1)%n;projecting=0
                elif terminal_only and e*(e-1)%n==0:
                    if terminal_only:pending.append(e)
                elif projecting:
                    if terminal_only:pending.extend([e,expected_complement])
                    e=pow(e,exponent,n);exponent+=1
                else:
                    if terminal_only:pending.extend([e,expected_complement])
                    powered=pow(expected_power,exponent,n);expected_power=powered
                    projection=(powered-1)%n
                    if projection and math.gcd(projection,n)!=1:
                        e=projection;projecting=1;singularities+=1;singular_projection=projection
                    elif powered not in (0,1) and powered*powered%n==powered:
                        e=powered
                    else:e=(powered+1)*((n+1)//2)%n
                    exponent+=1
                if terminal_only:pending.extend([e,expected_complement])
        assert banks[0]==e and banks[1]==e*(e-1)%n, 'returned state/residual differs'
        assert banks[5]==exponent, 'exponent continuation lost'
        assert [banks[i] for i in (2,6,7,8,9,10,12)]==[expected_complement,seed,last_trivial,last_residual,expected_power,singular_projection,original[12]], 'retained bank changed'
        if member['ancestry']:assert banks[13]==e, 'returned reentry presentation differs'
        assert banks[11]==0, 'SIC signed reconstruction residual'
        if data.get('cumulative_product_frame'):
            returned_frame=member['product_frame_state']
            assert returned_frame['initialized']==frame_initialized, 'product frame readiness lost'
            if frame_initialized:
                assert [bank_value(word) for word in returned_frame['matrix']]==matrix, 'cumulative product operator changed'
                assert bank_value(returned_frame['left'])==frame_left and bank_value(returned_frame['right'])==frame_right, 'carried product frame changed'
                if 'anchor_left' in returned_frame:
                    assert bank_value(returned_frame['anchor_left'])==anchor_left and bank_value(returned_frame['anchor_right'])==anchor_right, 'carried product point changed'
        receipt=member['receipt'];steps=len(member['ancestry'])
        for field in (0,1,2):
            assert receipt[field]==baseline['receipt'][field]+steps, 'Y/bind/SIC counter mismatch'
        # A successful algebra-image fuse can return before the second SIC.
        extra=steps if data.get('compose_idempotent') else 0
        if terminal_only:extra=-steps
        if data.get('compose_idempotent') and receipt[3] in (1,8) and steps:
            image=0;last=member['ancestry'][-1]
            for label,address in zip(last['labels'],last['addresses']):
                image=(image*radix+(address*256+label if data.get('keep_fibres') else address))%n
            if proper_gcd(image) or proper_gcd(n+1-image) or last.get('frame_closed') or last.get('point_closed') or last.get('support_closed'):extra-=1
        assert receipt[8]==baseline['receipt'][8]+steps+extra, 'SIC operation count differs'
        assert [receipt[i] for i in (5,6,7)]==[restarts,projecting,singularities], 'retained idempotent continuation differs'
        assert member['continuation']==baseline['continuation'], 'retained Y node bank changed'
        assert conflicts==member['conflicts'] and gates==member['gate_admitted'] and time==member['t_admitted']
        assert local_changes+baseline.get('changed',0)==member['changed'];changed+=local_changes
        distinct.add(banks[0])
        if terminal_only:
            assert [bank_value(word) for word in member['deferred_source_arms']]==pending, 'deferred source witness bank changed'
            readout=data['terminal_factor_readout'];slot=3*u+op
            limit=readout['member'] if readout['closed'] else data['held']
            if slot<=limit:
                for wi,witness in enumerate(pending):
                    terminal_scanned+=1;divisor=math.gcd(witness,n)
                    if 1<divisor<n:
                        assert readout['closed'] and slot==readout['member'] and wi==readout['witness'], 'terminal source witness identity changed'
                        assert numeral(readout['divisor'])==divisor and numeral(readout['quotient'])==n//divisor
                        assert divisor*(n//divisor)==n and n%divisor==0 and data['paired_boundary']
                        closed.append({'dialect':u,'operation':op,'divisor_word':readout['divisor'],'quotient_word':readout['quotient'],'boundary':'terminal-only'})
                        break
                else:assert not (readout['closed'] and slot==readout['member']), 'terminal divisor missing'
        if receipt[3] in (1,8):
            p,q=banks[3],banks[4]
            assert 1<p<n and n%p==0 and n//p==q and p*q==n, 'factor product failed'
            closed.append({'dialect':u,'operation':op,'divisor_word':numeral_word(p),'quotient_word':numeral_word(q)})
        else:
            assert receipt[3]==0, 'error status'
            assert banks[3:5]==original[3:5], 'pending paired source banks changed'
    if terminal_only and not partial:assert terminal_scanned==data['terminal_witnesses_checked'], 'terminal witness count changed'
    if not partial:assert bool(closed)==data['closed'], 'factor status differs from returned banks'
    return {'source_bits':n.bit_length(),'members':len(seen),'transitions_checked':transitions,
            'state_changes':changed,'distinct_returned_candidates':len(distinct),
            'preimage_and_source_recovery':True,'exact_residuals':True,
            'Y_and_banks_retained':True,'closed_pairs':closed,'product_frames_checked':frames,
            'unit_product_frames':units,'changed_product_frames':frame_changes,'product_point_updates':point_updates,'support_evaluations':support_calls,'support_reseeds':support_reseeds,'promotion_steps':promotion_steps,'promotion_changed_coordinates':promotion_changes,'terminal_witnesses_checked':terminal_scanned,'two_point_support_evaluations':secant_calls,'two_point_changed_operand_updates':secant_changed}


def verify_promotion_tables(tables):
    axes=tables['axis_values'];cards=list(map(len,axes));strides=[math.prod(cards[i+1:]) for i in range(12)]
    target=tables['promotion_target_indices']
    assert math.prod(cards)==tables['crystal_radix']
    assert sum(x*y for x,y in zip(target,strides))==tables['promotion_target_address']
    checked=0
    for dialect in tables['tables']:
        for operation,certificates in zip(dialect['operations'],dialect['promotion_certificates']):
            assert len(operation)==len(certificates)==256
            for cell,certificate in zip(operation,certificates):
                indices=certificate['source_indices']
                assert len(indices)==12 and all(0<=i<c for i,c in zip(indices,cards))
                assert sum(x*y for x,y in zip(indices,strides))==cell[0]
                expected=[{'from':axes[a][x]['glyph'],'to':axes[a][y]['glyph'],
                           'ordinal_delta':axes[a][y]['ordinal']-axes[a][x]['ordinal']}
                          for a,(x,y) in enumerate(zip(indices,target)) if x!=y]
                assert certificate['changes']==expected, 'kernel promotion signature differs from canonical ordinal tables'
                checked+=1
    return checked


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project',type=Path)
    parser.add_argument('receipt',type=Path)
    parser.add_argument('output',type=Path)
    parser.add_argument('--negative-controls',action='store_true')
    args=parser.parse_args()
    tables=json.loads((args.project/'dialect_tables.json').read_text())
    prepared=json.loads((args.project/'prepared.json').read_text())
    promotion_table_cells=verify_promotion_tables(tables) if 'promotion_target_indices' in tables else 0
    control_member=None
    if args.receipt.stat().st_size>128*1024*1024:
        with args.receipt.open('rb') as stream,mmap.mmap(stream.fileno(),0,access=mmap.ACCESS_READ) as raw:
            marker=b',"members":[';position=raw.find(marker)
            assert position>=0, 'missing member array'
            data=json.loads(raw[:position]+b'}');position+=len(marker)
            prior,parent_count=verified_parent(data,tables,prepared)
            result={'source_bits':tables['source_bits'],'members':0,'transitions_checked':0,
                    'state_changes':0,'preimage_and_source_recovery':True,'exact_residuals':True,
                    'Y_and_banks_retained':True,'closed_pairs':[], 'product_frames_checked':0,
                    'unit_product_frames':0,'changed_product_frames':0,'product_point_updates':0,'support_evaluations':0,'support_reseeds':0,'promotion_steps':0,'promotion_changed_coordinates':0,'terminal_witnesses_checked':0,'two_point_support_evaluations':0,'two_point_changed_operand_updates':0,'parent_transitions_checked':parent_count}
            identities=set();candidates=set()
            while position<raw.size()-3:
                next_position=raw.find(b',{"dialect":',position)
                end=next_position if next_position>=0 else raw.rfind(b']}')
                member=json.loads(raw[position:end])
                if control_member is None:control_member=member
                assert (member['dialect'],member['operation']) not in identities
                identities.add((member['dialect'],member['operation']))
                local=verify(dict(data,members=[member]),tables,prepared,partial=True,prior=prior)
                for key in ('members','transitions_checked','state_changes','product_frames_checked','unit_product_frames','changed_product_frames','product_point_updates','support_evaluations','support_reseeds','promotion_steps','promotion_changed_coordinates','terminal_witnesses_checked','two_point_support_evaluations','two_point_changed_operand_updates'):result[key]+=local[key]
                result['closed_pairs']+=local['closed_pairs']
                candidates.add(bank_value(member['state'][:tables['source_bits']+10]))
                if next_position<0:break
                position=next_position+1
            assert result['members']==data['held'], 'incomplete returned family'
            assert bool(result['closed_pairs'])==data['closed']
            result['distinct_returned_candidates']=len(candidates)
            if data.get('terminal_only_source_readout'):assert result['terminal_witnesses_checked']==data['terminal_witnesses_checked'], 'terminal witness count changed'
    else:
        data=json.loads(args.receipt.read_text())
        control_member=data['members'][0]
        prior,parent_count=verified_parent(data,tables,prepared)
        result=verify(data,tables,prepared,prior=prior);result['parent_transitions_checked']=parent_count
    if args.negative_controls:
        controls={
          'altered_dialect_image':lambda d:d['members'][0]['ancestry'][0]['addresses'].__setitem__(0,0),
          'altered_source_fibre':lambda d:d['members'][0]['ancestry'][0]['labels'].__setitem__(0,d['members'][0]['ancestry'][0]['labels'][0]^16),
          'lost_Y_continuation':lambda d:d['members'][0]['continuation'].__setitem__(0,999),
          'altered_returned_residual':lambda d:d['members'][0]['state'].__setitem__(tables['source_bits']+10,8 if d['members'][0]['state'][tables['source_bits']+10]==9 else 9),
          'altered_pending_source_bank':lambda d:d['members'][0]['state'].__setitem__(3*(tables['source_bits']+10),8 if d['members'][0]['state'][3*(tables['source_bits']+10)]==9 else 9),
        }
        result['negative_controls']={}
        if data.get('product_frame'):
            controls['altered_product_frame']=lambda d:d['members'][0]['ancestry'][0]['frame_left'].__setitem__(0,8 if d['members'][0]['ancestry'][0]['frame_left'][0]==9 else 9)
        if data.get('cofactor_feedback'):
            controls['altered_product_anchor']=lambda d:d['members'][0]['product_frame_state']['anchor_left'].__setitem__(0,8 if d['members'][0]['product_frame_state']['anchor_left'][0]==9 else 9)
        if data.get('support_secant'):
            controls['altered_divided_support']=lambda d:d['members'][0]['ancestry'][0]['support_divided'].__setitem__(0,8 if d['members'][0]['ancestry'][0]['support_divided'][0]==9 else 9)
            controls['altered_other_support_value']=lambda d:d['members'][0]['ancestry'][0]['support_other_value'].__setitem__(0,8 if d['members'][0]['ancestry'][0]['support_other_value'][0]==9 else 9)
        if data.get('terminal_only_frame_recovery'):
            controls['inflight_frame_recovery']=lambda d:d['members'][0]['ancestry'][0].__setitem__('frame_checked',True)
        if data.get('terminal_only_source_readout'):
            controls['inflight_source_readout']=lambda d:d.__setitem__('transport_source_probe_calls',1)
            if 'terminal_unchanged_members' in data:
                controls['perturbed_terminal_family']=lambda d:d.__setitem__('terminal_unchanged_members',0)
            controls['altered_deferred_witness']=lambda d:d['members'][0]['deferred_source_arms'][0].__setitem__(0,8 if d['members'][0]['deferred_source_arms'][0][0]==9 else 9)
        if data.get('promotion_return'):
            controls['altered_promotion_certificate']=lambda d:d['members'][0]['ancestry'][0]['promoted_addresses'].__setitem__(0,d['members'][0]['ancestry'][0]['promoted_addresses'][0]^1)
            controls['altered_promotion_operand']=lambda d:d['members'][0]['ancestry'][0]['promotion_point'].__setitem__(0,8 if d['members'][0]['ancestry'][0]['promotion_point'][0]==9 else 9)
            controls['altered_promotion_axis']=lambda d:d['members'][0]['ancestry'][0].__setitem__('promotion_axis',(d['members'][0]['ancestry'][0]['promotion_axis']+1)%12)
        if data.get('support_newton'):
            controls['altered_support_derivative']=lambda d:d['members'][0]['ancestry'][0]['support_derivative'].__setitem__(0,8 if d['members'][0]['ancestry'][0]['support_derivative'][0]==9 else 9)
        for name,change in controls.items():
            altered=dict(data,members=[copy.deepcopy(control_member)]);change(altered)
            try:verify(altered,tables,prepared,partial=True,prior=prior)
            except AssertionError as error:result['negative_controls'][name]={'rejected':True,'reason':str(error)}
            else:raise AssertionError('negative control accepted: '+name)
    if promotion_table_cells:result['promotion_table_cells_checked']=promotion_table_cells
    digest=hashlib.sha256()
    with args.receipt.open('rb') as stream:
        for chunk in iter(lambda:stream.read(1024*1024),b''):digest.update(chunk)
    result['receipt_sha256']=digest.hexdigest()
    result['verifier_sha256']=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
    for key in ('transport_source_probe_calls','terminal_sic_readbacks','terminal_unchanged_members','terminal_frame_readbacks'):
        if key in data:result[key]=data[key]
    args.output.write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n')
    print(json.dumps(result,indent=2,ensure_ascii=False))


if __name__=='__main__':main()
