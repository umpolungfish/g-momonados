#!/usr/bin/env python3
"""Audit a returned relational carrier, and independently check emitted factors."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import bisect
from array import array
from verify_dialect_factor import numeral


def audit_preparation_prefix(directory,report,prepared):
    compiled=json.loads((directory/'prepared.json').read_text())
    prefix=report.get('preparation_word_control');expected=compiled.get('control_prefix')
    if prefix is None and expected is None:return {'preparation_prefix_checked':False}
    if prefix!=expected:raise ValueError('executing preparation prefix differs from the compiled carrier')
    if prefix['word']!=report['control_word']['word'] or prefix['source_word']!=report['source_word']:
        raise ValueError('preparation prefix changed its word or source')
    if prefix['next_pc']!=12 or prefix['executed_positions']!=list(range(12)):
        raise ValueError('preparation prefix skipped or repeated an operation')
    steps=prefix['steps'];word=prefix['word']
    if len(steps)!=12:raise ValueError('preparation prefix lost operation evidence')
    phase=0;contractions=0;visits=0
    for pc,step in enumerate(steps):
        if step['pc']!=pc or step['glyph']!=word[pc] or step['phase_before']!=phase:
            raise ValueError('preparation prefix lost its executing position or phase')
        if step['contractions_before']!=contractions or step['product_visits_before']!=visits:
            raise ValueError('preparation prefix recreated an independent carrier')
        contractions=step['contractions_after'];visits=step['product_visits_after']
        if contractions<step['contractions_before'] or visits<step['product_visits_before']:
            raise ValueError('preparation prefix restarted its work counters')
        phase+=int(pc==9)
        if step['phase_after']!=phase:raise ValueError('preparation advance executed at the wrong word position')
        if step['pending_after']!=0 and pc in range(1,12):
            raise ValueError('compiled prefix left an operator pending on the prepared register')
    if prefix['phase']!=phase or prefix['carried_T_F_images']!=compiled['prepared_bridge_image']:
        raise ValueError('compiled prefix lost its carried native image')
    root=prefix['root']
    if type(root) is not int or not 0<=root<len(prepared) or prepared[root]!=1:
        raise ValueError('compiled prefix lost source truth at its product boundary')
    for field in ('entry_bank','entry_trail','entry_phase'):
        if report['holding_frame'][field]!=prefix['holding_frame'][field]:
            raise ValueError('compiled holding frame changed during execution')
    native=directory/'native_arithmetic.json'
    if native.exists():
        program=json.loads(native.read_text());advance=steps[9]
        before={kind:gate['stages'][0]['before'] for kind,gate in program['gates'].items()}
        after={}
        for kind,rows in before.items():
            right=program['gates'][kind]['stages'][0]['right']
            after[kind]=[[program['native_axis_tables'][0][axis][left[axis]*program['axis_cards'][axis]+rhs[axis]][0]
                          for axis in range(12)] for left,rhs in zip(rows,right)]
        if advance['native_types_before']!=before or advance['native_types_after']!=after:
            raise ValueError('compiled advance did not consume the preceding native tuple state')
    return {'preparation_prefix_checked':True,'preparation_operations_checked':12}


def audit_verification_tail(report, source, inputs, domains, bank_value, returned_state):
    tail=report.get('verification_word_control')
    if tail is None:return {'verification_tail_checked':False}
    pc=tail['pc'];closed=report['closure'] is not None
    if type(pc) is not int or (pc!=45 if closed else pc not in range(36,44)):
        raise ValueError('invalid verification-word continuation cursor')
    if tail['executed_positions']!=list(range(36,pc)):
        raise ValueError('verification word skipped or repeated an operation')
    n=numeral(source);p=numeral(tail['divisor_word']);paired=numeral(tail['paired_word'])
    selected=[0,0]
    for variable,wire in inputs.items():
        if domains[wire] not in (1,2):raise ValueError('verification word retained an unresolved factor slot')
        if domains[wire]==1:selected[variable%2]|=1<<(variable//2)
    if selected!=[p,paired]:raise ValueError('verification word differs from the executing factor lanes')
    if pc>36 and not 1<p<n:raise ValueError('verification word accepted an improper divisor')
    q=tail['quotient_word'];r=tail['remainder_word'];product=tail['product_word']
    if pc>37:
        if q is None or r is None or (numeral(q),numeral(r))!=divmod(n,p):
            raise ValueError('verification word lost its native quotient or remainder')
    elif q is not None or r is not None:raise ValueError('quotient appeared before its word operation')
    if pc>38 and numeral(r)!=0:raise ValueError('verification word accepted a nonzero remainder')
    if pc>39:
        if product is None or numeral(product)!=p*numeral(q) or numeral(product)!=n:
            raise ValueError('verification word lost its exact source product')
    elif product is not None:raise ValueError('product appeared before its word operation')
    if pc>40 and numeral(q)!=paired:raise ValueError('verification word lost its paired boundary')
    if pc>41 and product!=source:raise ValueError('verification word changed its canonical source identity')
    returned=tail['returned_bank'];holding=report.get('holding_frame')
    if holding is None:raise ValueError('verification word lost its enclosing frame')
    if pc>42:
        if type(returned) is not int or returned<0:raise ValueError('verification word lost its enclosing return bank')
        if any(bank_value(returned,slot)!=value for slot,value in enumerate(returned_state)):
            raise ValueError('verification word returned a different register')
    elif returned is not None:raise ValueError('register returned before the enclosing fuse')
    if closed:
        closure=report['closure']
        if holding['fixed'] is not True or holding['returned_bank']!=returned:
            raise ValueError('verification word lost fixation of its returned register')
        if closure['divisor_word']!=tail['divisor_word'] or closure['quotient_word']!=q:
            raise ValueError('terminal readout differs from the word return')
        clock=tail['trigger_to_verified_factor_seconds']
        if type(clock) not in (float,int) or not 0<=clock<=report['trigger_to_readout_seconds']:
            raise ValueError('verification word has an invalid factor clock')
        if closure['trigger_to_verified_factor_seconds']!=clock:
            raise ValueError('terminal factor clock differs from the fixed word return')
    elif holding['fixed'] is not False or holding['returned_bank'] is not None or tail['trigger_to_verified_factor_seconds'] is not None:
        raise ValueError('verification word fixed an open continuation')
    return {'verification_tail_checked':True,'verification_tail_pc':pc}


def verify(directory, altered_report=None, bank_storage='mapped'):
    report = altered_report if altered_report is not None else json.loads((directory / 'return.json').read_text())
    bank_description=report.get('register_banks',[[0,'']])
    if report.get('control_word') and report['control_word'].get('full_word_execution') and report['closure'] is None:
        raise ValueError('open computation claimed a complete word return')
    from packed_relational_return import unpack,PackedRecords,mapped,file_digest
    report=unpack(report)
    binding = json.loads((directory / 'binding.json').read_text())
    if binding.get('first_stdout_byte_seconds') is not None and binding['first_stdout_byte_seconds']<report['trigger_to_readout_seconds']:
        raise ValueError('output arrived before the terminal readout boundary')
    source = (directory / 'source.imasm').read_text().strip()
    if report['source_word'] != source:
        raise ValueError('source bank changed')
    for filename, key in [('source.imasm', 'source_sha256'), ('membrane', 'executable_sha256'),
                          ('return.json', 'return_sha256'), ('relation.ops', 'relation_sha256'),
                          ('register.cells', 'prepared_register_sha256'),
                          ('prepared.json','prepared_control_sha256'),
                          ('relation.cells', 'joint_cells_sha256'), ('cell.rows', 'native_cell_rows_sha256')]:
        if key in binding and file_digest(directory/filename) != binding[key]:
            raise ValueError(f'changed bound artifact: {filename}')
    ops = PackedRecords(directory/'relation.ops','<IIII')
    prepared = list((directory / 'register.cells').read_bytes())
    domains = report['register']
    if not (len(ops) == len(prepared) == len(domains)) or any(d not in [1, 2, 3] for d in domains):
        raise ValueError('invalid executing register dimensions or alternatives')
    inputs = {}
    prefix_audit=audit_preparation_prefix(directory,report,prepared)
    joint_cells = []
    if (directory / 'relation.cells').exists():
        joint_cells = list(struct.iter_unpack('<IIIII', (directory / 'relation.cells').read_bytes()))
        if joint_cells:
            from analyze_product_cell_fusion import cells
            expected_ports = [ports for _, ports in cells(ops)]
            if joint_cells != expected_ports:
                raise ValueError('joint cell lost its actual lowered arithmetic ports')
            del expected_ports
        rows = (directory / 'cell.rows').read_bytes()
        expected_rows = []
        for index in range(8):
            bits = [(index >> i) & 1 for i in range(3)]
            expected_rows.extend([1 if b else 2 for b in bits] +
                                 [1 if sum(bits) % 2 else 2, 1 if sum(bits) >= 2 else 2])
        if list(rows) != expected_rows:
            raise ValueError('native joint sum/carry relation changed')
        if report.get('joint_cells', 0) != len(joint_cells):
            raise ValueError('executing joint cell bank differs')
    for wire, (op, a, b, required) in enumerate(ops):
        if op not in [0, 3, 4, 5, 7, 8]:
            raise ValueError('unknown relation operator')
        if op in [3, 4, 7] and not (a < wire and (op == 3 or b < wire)):
            raise ValueError('relation lost its retained dependency graph')
        if op == 8:
            inputs[a] = wire
        if (op == 0 and domains[wire] != 2) or ((op == 5 or required) and domains[wire] != 1):
            raise ValueError('source boundary or constant changed')
    if sorted(inputs) != list(range(len(inputs))):
        raise ValueError('unknown factor lanes incomplete')
    native_audit={};prepared_state=prepared;returned_state=domains
    if report.get('resident_dialect') is not None:
        import gzip
        native_prepared=mapped(directory/'correlated.cells')
        saved=report['resident_dialect']
        native_returned=(gzip.decompress(bytes.fromhex(saved['state_gzip_hex'])) if 'state_gzip_hex' in saved
                         else bytes.fromhex(saved['state_hex']))
        prepared_state=bytearray(prepared);prepared_state.extend(memoryview(native_prepared))
        returned_state=bytearray(domains);returned_state.extend(native_returned)
        del native_prepared,native_returned
    restored = returned_state[:]
    for wire, previous in reversed(report['continuation']['trail']):
        if not (0 <= wire < len(restored)) or not 0<=previous<=255 or (wire<len(ops) and previous not in [1,2,3]) or restored[wire] & ~previous:
            raise ValueError('noncontracting or malformed continuation trail')
        restored[wire] = previous
    if restored != prepared_state:
        raise ValueError('continuation does not return to the actual prepared register')
    del restored
    if report.get('resident_dialect') is not None:
        from audit_resident_dialect import audit
        _,_,native_audit=audit(directory,report,ops,joint_cells,
            (memoryview(prepared_state)[len(ops):],memoryview(returned_state)[len(ops):]))
    at = 0
    for wire, alternative, boundary in report['continuation']['decisions']:
        if wire not in inputs.values() or alternative not in [0, 2] or not at <= boundary <= len(report['continuation']['trail']):
            raise ValueError('branch continuation changed')
        if domains[wire] != (1 if alternative else 2):
            raise ValueError('selected branch lost its correlated assignment')
        at = boundary
    for gate in report['continuation']['pending_gates']:
        if not 0 <= gate < len(ops) + len(joint_cells)+native_audit.get('correlated_pairs_checked',0) or (
                gate < len(ops) and ops[gate][0] not in [3, 4, 7]):
            raise ValueError('invalid pending relation operation')
    if binding.get('parent_return_sha256'):
        if file_digest(directory/'parent_return.json') != binding['parent_return_sha256']:
            raise ValueError('retained parent return changed')
        with (directory/'parent_return.json').open() as stream:parent=json.load(stream)
        if parent['status'] != 'held' or parent['source_word'] != source:
            raise ValueError('continuation parent is not the same open source')
        for counter in ['branches', 'contractions', 'rows_examined', 'bridge_applications']:
            if report[counter] < parent[counter]:
                raise ValueError('retained execution counters were restarted')
        del parent
    provenance = json.loads((directory / 'bridge_provenance.json').read_text())
    if 'tables_path' in provenance:
        from bake_relational_imasm_bridge import bridges
        _, independently_rebuilt = bridges(Path(provenance['tables_path']),
                                           Path(provenance['native_execution_receipt_path']))
        if provenance != independently_rebuilt:
            raise ValueError('native rule action or bridge provenance changed')
    image = [1, 2]
    phase = report['bridge_phase']
    hops = [hop['native_hop_image'] for hop in provenance['ancestry']]
    seen = {}
    tick = 0
    while tick < phase:
        key = (tick % len(hops), tuple(image))
        if key in seen:
            period = tick - seen[key]
            skip = (phase - tick) // period
            if skip:
                tick += skip * period
                continue
        seen[key] = tick
        image = [hops[tick % len(hops)][value] for value in image]
        tick += 1
    if image != report['carried_T_F_images']:
        raise ValueError('bridge effects were not carried across hops')
    history, cycles = [], {}
    current = [1, 2]
    while (len(history) % len(hops), tuple(current)) not in cycles:
        cycles[(len(history) % len(hops), tuple(current))] = len(history)
        history.append(current)
        current = [hops[(len(history)-1) % len(hops)][value] for value in current]
    cycle_start = cycles[(len(history) % len(hops), tuple(current))]
    def image_at(tick):
        if tick >= len(history):
            tick = cycle_start + (tick-cycle_start) % (len(history)-cycle_start)
        return history[tick]
    banks = report.get('register_banks', [[0, '']])
    if not banks or banks[0] != [0, '']:
        raise ValueError('prepared register bank root changed')
    bank_store=None
    if bank_storage=='mapped':
        from resident_bank_audit_store import MappedBankStore,input_binding,json_digest
        bank_binding=input_binding(bank_description,prepared_state,report.get('bank_value_bits',2),Path(__file__))
        bank_store=MappedBankStore(directory/'bank_audit'/json_digest(bank_binding),bank_binding)
        parents,decoded,keys=bank_store.parents,bank_store.values,bank_store.keys
        if len(parents)>len(banks):raise ValueError('decoded bank cache exceeds its source prefix')
    elif bank_storage=='memory':
        parents=array('Q',[0]);decoded=[bytearray()];keys=[array('Q')]
    else:raise ValueError('unknown decoded bank storage')
    def bank_value(bank, wire):
        while bank:
            at = bisect.bisect_left(keys[bank], wire)
            if at < len(keys[bank]) and keys[bank][at] == wire:
                return decoded[bank][at]
            bank = parents[bank]
        return prepared_state[wire]
    for bank in range(len(parents),len(banks)):
        parent,encoded=banks[bank]
        if not 0 <= parent < bank:
            raise ValueError('register bank ancestry changed')
        data = bytes.fromhex(encoded)
        if report.get("register_banks_encoding") == "gzip-hex":
            import gzip
            data = gzip.decompress(data)
        previous = code = shift = 0
        delta = bytearray()
        wires = array("Q")
        for byte in data:
            code |= (byte & 127) << shift
            if byte & 128:
                shift += 7
                if shift >= 64:
                    raise ValueError('register bank varint overflow')
                continue
            bits=report.get('bank_value_bits',2)
            gap, mask = code >> bits, code & ((1<<bits)-1)
            wire = previous + gap - 1
            if gap == 0 or not 0 <= wire < len(prepared_state) or mask & ~bank_value(parent, wire):
                raise ValueError('register bank is not a source-bound contraction')
            wires.append(wire)
            delta.append(mask)
            previous, code, shift = wire + 1, 0, 0
        if shift:
            raise ValueError('register bank ends inside an encoded cell')
        if bank_store is not None:bank_store.append(parent,wires,delta)
        else:
            parents.append(parent);decoded.append(delta);keys.append(wires)
    def check_child(base, child, wire, literal, consistent):
        if not 0 <= base < child < len(banks) or parents[child] != base:
            raise ValueError('nested children lost their common resident parent')
        if bank_value(base, wire) != 3:
            raise ValueError('nested split did not start from both alternatives')
        value = bank_value(child, wire)
        if value & ~literal or (consistent and value != literal):
            raise ValueError('nested child lost its source assumption')
        contradiction_slots=len(ops)+native_audit.get('native_frames_checked',0)
        if any(mask == 0 and wire<contradiction_slots for wire,mask in zip(keys[child],decoded[child])) == consistent:
            raise ValueError('nested consistency evidence differs from its bank')
    common_count = factor_count = 0
    for entry in report.get('nested_receipts', []):
        wire, base, first, second, first_ok, second_ok, tick, readout, changes = entry
        if wire not in inputs.values() or tick > phase or readout != image_at(tick):
            raise ValueError('nested operation provenance changed')
        check_child(base, first, wire, 1, first_ok)
        check_child(base, second, wire, 2, second_ok)
        seen_changes = set()
        for target, mask in changes:
            expected = (bank_value(first, target) if first_ok else 0) | (
                bank_value(second, target) if second_ok else 0)
            if target in seen_changes or mask != expected or mask == bank_value(base, target):
                raise ValueError('nested fuse changed a common consequence')
            seen_changes.add(target)
            common_count += 1
            factor_count += target<len(ops) and ops[target][0] == 8
    if common_count != report.get('nested_common_contractions', 0) or factor_count != report.get('nested_factor_bit_contractions', 0):
        raise ValueError('nested contraction counters differ from retained observations')
    active = report['continuation'].get('nested_frame')
    if active is not None:
        wire, boundary, base, stage, first, first_ok = active
        if wire not in inputs.values() or stage not in [0, 1] or not 0 <= boundary <= len(report['continuation']['trail']):
            raise ValueError('active nested continuation changed')
        if domains[wire] != (1 if stage == 0 else 2):
            raise ValueError('active nested child lost its assumption')
        parent_register = returned_state[:]
        for target, old in reversed(report['continuation']['trail'][boundary:]):
            parent_register[target] = old
        bank_register = prepared_state[:]
        if base:
            if not 0 < base < len(banks) or parents[base] != 0:
                raise ValueError('active parent bank is not a prepared-source overlay')
            for target, value in zip(keys[base], decoded[base]):
                bank_register[target] = value
        if parent_register != bank_register:
            raise ValueError('active re-entry did not retain its actual parent register')
        if stage:
            check_child(base, first, wire, 1, first_ok)
    inner = report.get('inner_word_control')
    if inner is not None:
        block=inner.get('block',0)
        allowed=range(19,26) if block==0 else range(30,36)
        if block not in (0,1) or type(inner['pc']) is not int or inner['pc'] not in allowed or type(inner['consistent']) is not bool:
            raise ValueError('invalid inner-word continuation cursor')
        if active is None and (inner['pc'] != (20 if block==0 else 30) or inner['consistent'] is not True):
            raise ValueError('inactive inner-word continuation retains unfinished child work')
    legacy=report.get('legacy_pending_work')
    if legacy is not None:
        if legacy.get('phase')!=phase or legacy.get('action')!='propagate_source_product':
            raise ValueError('legacy pending work changed its native phase or operation')
        if active is not None and (inner is None or inner['pc']!=19):
            raise ValueError('legacy child lost its pending operation before new word control')
        if report['control_word']['completed_dialect_blocks']!=0:
            raise ValueError('new dialect work ran before the retained legacy operation')
    reversals=report.get('reversal_receipts',[])
    for recovered in reversals:
        parent,before,after=(recovered[k] for k in ('parent_bank','before_bank','after_bank'))
        if not 0 <= parent < len(banks) or not parent <= before < len(banks) or not parent <= after < len(banks):
            raise ValueError('protected reversal lost its retained child bank')
        if before==parent or after==parent:
            if (parent,before,after)!=(0,0,0) or recovered['cleared_slots']!=0:
                raise ValueError('nonempty reversal lost its overlay bank')
        if parents[before]!=parent or parents[after]!=parent:
            raise ValueError('protected reversal changed the common parent')
        for slot in set(keys[before]) | set(keys[after]):
            if bank_value(before,slot)!=bank_value(after,slot):
                raise ValueError('protected reversal lost a wire, row or pair')
        if recovered['correlated_state_recovered'] is not True or not 0 <= recovered['phase'] <= phase or recovered['image']!=image_at(recovered['phase']):
            raise ValueError('protected reversal changed its carried phase')
        if not 0 <= recovered['trail_boundary'] <= recovered['trail_length'] or recovered['cleared_slots']<0 or recovered['pending_gates']<0:
            raise ValueError('protected reversal lost its live continuation')
        split=recovered.get('split_recovery')
        if split is not None:
            wire=split['wire']
            if wire not in inputs.values() or split['cleared']!=bank_value(parent,wire) or split['cleared']!=3 or split['recovered']!=bank_value(after,wire) or split['recovered'] not in (1,2):
                raise ValueError('protected reversal lost its split assumption')
        if recovered.get('word_pc') is not None and recovered['word_pc'] not in (18,27,33):
            raise ValueError('protected reversal has an invalid word position')
    outer=report.get('outer_word_control')
    if outer is not None:
        pc=outer['pc']
        if type(pc) is not int or pc not in (18,19,26,27,28,29):
            raise ValueError('invalid surrounding-word continuation cursor')
        if type(outer['completed_interblock_seams']) is not int or outer['completed_interblock_seams']<0:
            raise ValueError('invalid surrounding-word execution count')
        if inner is not None:
            allowed=(18,19) if inner.get('block',0)==0 else (26,27,28,29)
            if pc not in allowed or (active is not None and pc!=(19 if inner.get('block',0)==0 else 29)):
                raise ValueError('surrounding-word cursor lost its nested child')
    closure = report['closure']
    holding = report.get('holding_frame')
    if holding is not None:
        entry = holding['entry_bank']
        if type(entry) is not int or not 0 <= entry < len(banks) or not 0 <= holding['entry_phase'] <= report['bridge_phase']:
            raise ValueError('enclosing holding frame lost its entry bank or phase')
        if type(holding['entry_trail']) is not int or holding['entry_trail'] < 0:
            raise ValueError('enclosing holding frame has an invalid entry continuation')
        if closure is None:
            if holding['fixed'] is not False or holding['returned_bank'] is not None:
                raise ValueError('open computation was fixed or returned before verification')
        else:
            selected = holding['returned_bank']
            if holding['fixed'] is not True or type(selected) is not int or not 0 <= selected < len(banks):
                raise ValueError('verified closure lost its returned and fixed holding frame')
            if any(bank_value(selected, slot) != value for slot, value in enumerate(returned_state)):
                raise ValueError('returned holding bank differs from the executing wire/row/pair register')
    tail_audit=audit_verification_tail(report,source,inputs,domains,bank_value,returned_state)
    full_word=False
    control=report.get('control_word')
    if control is not None:
        positions={entry.get('word_pc') for entry in reversals}
        full_word=bool(prefix_audit['preparation_prefix_checked'] and closure is not None
            and tail_audit.get('verification_tail_pc')==45 and control['completed_dialect_blocks']>0
            and outer is not None and outer['completed_interblock_seams']>0
            and inner is not None and inner.get('block',0)==0 and active is None
            and len(report.get('nested_receipts',[]))>=2 and {18,27,33}<=positions)
        if control['full_word_execution'] is not full_word:
            raise ValueError('full-word execution claim differs from its compiled, nested and terminal evidence')
    values_checked = 0
    if closure is not None:
        n = numeral(source)
        p = numeral(closure['divisor_word'])
        q = numeral(closure['quotient_word'])
        if not (1 < p < n and n % p == 0 and n // p == q and p*q == n):
            raise ValueError('emitted divisor or exact source product is invalid')
        values = []
        for wire, (op, a, b, required) in enumerate(ops):
            value = {0: False, 5: True}.get(op)
            if op == 8:
                value = bool(((p if a % 2 == 0 else q) >> (a//2)) & 1)
            elif op == 3:
                value = not values[a]
            elif op == 4:
                value = values[a] and values[b]
            elif op == 7:
                value = values[a] or values[b]
            if not domains[wire] & (1 if value else 2) or (required and not value):
                raise ValueError('emitted pair violates a retained source relation')
            values.append(value)
        values_checked = len(values)
    from audit_resident_product_return import audit as audit_product
    product_audit=audit_product(directory,report)
    from audit_resident_readers import audit as audit_readers
    reader_audit=audit_readers(directory,report,binding,ops,joint_cells)
    return {**native_audit,**product_audit,**reader_audit,**prefix_audit,**tail_audit,'full_word_execution':full_word,'source_bits': numeral(source).bit_length(), 'status': report['status'],
            'proper_factor': closure is not None, 'holding_frame_checked': holding is not None, 'inner_word_cursor_checked': inner is not None, 'outer_word_cursor_checked': outer is not None, 'protected_reversals_checked': len(reversals), 'emitted_assignment_gate_checks': values_checked,
            'retained_wires': len(ops), 'prepared_register_recovered_from_trail': True,
            'joint_cells_checked': len(joint_cells),
            'register_banks_checked': len(banks), 'nested_fuses_checked': len(report.get('nested_receipts', [])),
            'nested_factor_bit_contractions_checked': factor_count,
            'branch_continuation_checked': True, 'carried_native_bridge_image_checked': True,
            'bank_audit_storage':bank_store.receipt() if bank_store is not None else {'kind':'memory'},
            'scope': 'assignment and continuation audit',
            'return_sha256': file_digest(directory/'return.json')}


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('directory', type=Path)
    p.add_argument('output', type=Path)
    p.add_argument('--bank-storage',choices=['mapped','memory'],default='mapped',
                   help='decoded bank representation; memory is the reference audit instrument')
    args = p.parse_args()
    result = verify(args.directory,bank_storage=args.bank_storage)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result))
