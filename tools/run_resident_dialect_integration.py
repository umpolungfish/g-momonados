#!/usr/bin/env python3
"""Run source-only resident dialect preparation, execution and independent audit."""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import secrets
import subprocess
import sys
import time

ROOT=Path(__file__).resolve().parents[2]
TOOLS=Path(__file__).resolve().parent
TABLES=Path('/mnt/d/imsgct-progress/artifacts/dialect_small_controls_fixed_20261018/n187.source/dialect_tables.json')
NATIVE=Path('/mnt/d/imsgct-progress/artifacts/imasm_type_bridge_current_20261018/execution.json')


def run(stage,base,product=False,parent_return=None,nested=False):
    if stage=='audit':audit_output(base);return
    from verify_dialect_factor import numeral_word
    if stage=='control':
        from sympy import isprime
        inputs=base/'inputs';references=base/'verification_only';inputs.mkdir(parents=True,exist_ok=True);references.mkdir(exist_ok=True)
        source=inputs/'fresh_balanced_208.imasm'
        if source.exists():raise ValueError('fresh control already exists; use its continuation')
        def prime():
            while True:
                value=secrets.randbits(104)|(1<<103)|1
                if isprime(value):return value
        while True:
            p,q=prime(),prime();n=p*q
            if p!=q and n.bit_length()==208:break
        source.write_text(numeral_word(n)+'\n')
        (references/'fresh_balanced_208.json').write_text(json.dumps({'p':str(p),'q':str(q),'source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),
            'scope':'verification only; never passed to the compiler or membrane'},indent=2)+'\n')
        output=base/'fresh208';seconds=30;resume=None
    elif stage=='resume':
        source=base/'inputs/fresh_balanced_208.imasm';output=base/'fresh208_resume';seconds=2
        resume=base/'fresh208/return.json'
    else:
        source=ROOT/'G-mOMonadOS/measurements/resident_imasm_integration_20261012/inputs/fresh_balanced_512.imasm'
        output=base/'locked512';seconds=0;resume=parent_return
    command=[sys.executable,str(TOOLS/'bake_relational_imasm_bridge.py'),str(source),str(output),
             '--tables',str(TABLES),'--native-bridges',str(NATIVE),'--joint-cells','--resident-dialects',
             '--seconds',str(seconds)]
    if resume:command+=['--resume',str(resume)]
    if product:command+=['--source-product-return']
    if nested:command+=['--nested-reentry']
    subprocess.run(command,check=True)
    reference=base/'verification_only/fresh_balanced_208.json'
    if reference.exists():
        from check_prepared_factor_witness import check
        retained=check(output,reference)
        (output/'prepared_reference_check.json').write_text(json.dumps(retained,indent=2)+'\n')
    audit_output(output,stage)


def audit_output(output,stage='audit'):
    from packed_relational_return import file_digest
    certificate=output/'verification.json'
    if not certificate.exists():
        subprocess.run([sys.executable,str(TOOLS/'verify_relational_imasm_bridge.py'),str(output),str(certificate)],check=True)
    else:
        verified=json.loads(certificate.read_text());binding=json.loads((output/'binding.json').read_text())
        if binding.get('prepared_control_sha256') and file_digest(output/'prepared.json')!=binding['prepared_control_sha256']:
            raise ValueError('changed previously verified prepared word prefix')
        for filename,key in [('source.imasm','source_sha256'),('membrane','executable_sha256'),
                             ('return.json','return_sha256'),('relation.ops','relation_sha256'),
                             ('register.cells','prepared_register_sha256'),('relation.cells','joint_cells_sha256'),
                             ('cell.rows','native_cell_rows_sha256')]:
            if file_digest(output/filename)!=binding[key]:raise ValueError('changed previously verified artifact')
        for filename,expected in binding.get('correlated_artifacts',{}).items():
            if file_digest(output/filename)!=expected:raise ValueError('changed previously verified joint bank')
        if verified['return_sha256']!=binding['return_sha256']:raise ValueError('verification binds another return')
    # Altered native rule banks and changed packed continuation bytes must be
    # rejected by the same independent audit that accepts the original.
    from verify_relational_imasm_bridge import verify
    negative=[]
    original=json.loads((output/'return.json').read_text())
    labels=['native_type_bank','trail_previous_mask','source_boundary']
    if original.get('holding_frame') is not None:labels.append('premature_fixation')
    if original.get('inner_word_control') is not None:labels.append('inner_word_cursor')
    if original.get('outer_word_control') is not None:labels.append('outer_word_cursor')
    if original.get('preparation_word_control') is not None:labels.append('preparation_cursor')
    if original.get('control_word') is not None:labels.append('full_word_claim')
    reversals=original.get('reversal_receipts',[])
    if (reversals.get('records',0) if isinstance(reversals,dict) else len(reversals)):labels.append('reversal_recovery')
    for label in labels:
        changed=copy.deepcopy(original)
        if label=='native_type_bank':changed['resident_dialect']['native_types']['4'][0][0]^=1
        elif label=='trail_previous_mask':
            import gzip,struct
            c=changed['continuation'];raw=bytearray(gzip.decompress(bytes.fromhex(c['trail_gzip_hex'])))
            if not raw:raise ValueError('execution did not exercise continuation')
            # A previous empty mask cannot contain the executing contraction.
            raw[8]=0;c['trail_gzip_hex']=gzip.compress(raw).hex()
        elif label=='reversal_recovery':
            from packed_relational_return import archived
            changed['reversal_receipts']=list(archived(changed['reversal_receipts']))
            record=next((row for row in reversed(changed['reversal_receipts']) if row.get('split_recovery') is not None),None)
            if record is None:changed['reversal_receipts'][-1]['after_bank']=-1
            else:record['split_recovery']['cleared']=1
        elif label=='inner_word_cursor':changed['inner_word_control']['pc']=43
        elif label=='outer_word_cursor':changed['outer_word_control']['pc']=43
        elif label=='preparation_cursor':changed['preparation_word_control']['next_pc']=9
        elif label=='full_word_claim':changed['control_word']['full_word_execution']=not changed['control_word']['full_word_execution']
        elif label=='premature_fixation':changed['holding_frame']['fixed']=True
        else:
            # A source alteration tests the actual immutable source boundary.
            changed['source_word']+='⊙'
        try:verify(output,changed)
        except (ValueError,AssertionError,IndexError,KeyError):negative.append({'control':label,'rejected':True})
        else:raise ValueError('altered resident evidence was accepted: '+label)
    (output/'negative_controls.json').write_text(json.dumps(negative,indent=2)+'\n')
    if original.get('source_product_return') is not None:
        from audit_resident_product_return import audit
        from verify_dialect_factor import numeral,numeral_word
        product_negative=[]
        for label in ('native_quotient_return','possible_lane_removed'):
            changed=copy.deepcopy(original)
            from packed_relational_return import archived
            # The production return names an immutable archive prefix. Mutate
            # a private decoded copy for the negative check, never the archive.
            changed['source_product_return']['banks']=list(archived(changed['source_product_return']['banks']))
            item=changed['source_product_return']['banks'][-1]
            if label=='native_quotient_return':
                entry=item['bank']['quotient_returns'][0];entry['remainder']=numeral_word(numeral(entry['remainder'])+1)
            else:
                bit=next(i for i,v in enumerate(item['p_after']) if v==3);item['p_after'][bit]=0
            try:audit(output,changed)
            except ValueError as error:product_negative.append({'control':label,'rejected':True,'reason':str(error)})
            else:raise ValueError('changed source product return was accepted')
        (output/'product_negative_controls.json').write_text(json.dumps(product_negative,indent=2)+'\n')
    if original.get('prepared_dependency_incidence'):
        import struct
        from packed_relational_return import PackedRecords
        from audit_resident_readers import audit as audit_readers
        raw=bytearray((output/'reader.index').read_bytes());wires=struct.unpack_from('<Q',raw)[0]
        struct.pack_into('<Q',raw,8+8*(wires+1),0)
        binding=json.loads((output/'binding.json').read_text())
        ops=PackedRecords(output/'relation.ops','<IIII')
        cells=list(struct.iter_unpack('<IIIII',(output/'relation.cells').read_bytes()))
        try:audit_readers(output,original,binding,ops,cells,raw)
        except ValueError as error:reader_negative=[{'control':'wrong_dependency_incidence','rejected':True,'reason':str(error)}]
        else:raise ValueError('wrong source dependency incidence was accepted')
        (output/'reader_negative_controls.json').write_text(json.dumps(reader_negative,indent=2)+'\n')
    print(json.dumps({'stage':stage,'directory':str(output),'negative_controls':negative}))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('stage',choices=['control','resume','locked','audit'])
    p.add_argument('--output',type=Path,default=Path('/mnt/d/imsgct-progress/artifacts/resident_dialect_integration_20261018'))
    p.add_argument('--source-product-return',action='store_true')
    p.add_argument('--nested-reentry',action='store_true')
    p.add_argument('--parent-return',type=Path)
    args=p.parse_args();run(args.stage,args.output,args.source_product_return,args.parent_return,args.nested_reentry)
