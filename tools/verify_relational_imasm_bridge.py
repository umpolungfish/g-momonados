#!/usr/bin/env python3
"""Audit a returned relational carrier, and independently check emitted factors."""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import bisect
import itertools
from array import array
from verify_dialect_factor import numeral


def verify(directory, altered_report=None):
    report = altered_report if altered_report is not None else json.loads((directory / 'return.json').read_text())
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
                          ('relation.cells', 'joint_cells_sha256'), ('cell.rows', 'native_cell_rows_sha256')]:
        if key in binding and file_digest(directory/filename) != binding[key]:
            raise ValueError(f'changed bound artifact: {filename}')
    ops = PackedRecords(directory/'relation.ops','<IIII')
    prepared = list((directory / 'register.cells').read_bytes())
    domains = report['register']
    if not (len(ops) == len(prepared) == len(domains)) or any(d not in [1, 2, 3] for d in domains):
        raise ValueError('invalid executing register dimensions or alternatives')
    inputs = {}
    joint_cells = []
    if (directory / 'relation.cells').exists():
        joint_cells = list(struct.iter_unpack('<IIIII', (directory / 'relation.cells').read_bytes()))
        if joint_cells:
            from analyze_product_cell_fusion import cells
            expected_ports = [ports for _, ports in cells(ops)]
            if joint_cells != expected_ports:
                raise ValueError('joint cell lost its actual lowered arithmetic ports')
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
    parents = array("Q",[0])
    decoded = [bytearray()]
    keys = [array("I")]
    def bank_value(bank, wire):
        while bank:
            at = bisect.bisect_left(keys[bank], wire)
            if at < len(keys[bank]) and keys[bank][at] == wire:
                return decoded[bank][at]
            bank = parents[bank]
        return prepared_state[wire]
    for bank, (parent, encoded) in enumerate(itertools.islice(banks,1,None), 1):
        if not 0 <= parent < bank:
            raise ValueError('register bank ancestry changed')
        data = bytes.fromhex(encoded)
        if report.get("register_banks_encoding") == "gzip-hex":
            import gzip
            data = gzip.decompress(data)
        previous = code = shift = 0
        delta = bytearray()
        wires = array("I")
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
        parents.append(parent)
        decoded.append(delta)
        keys.append(wires)
    def check_child(base, child, wire, literal, consistent):
        if not 0 <= base < child < len(banks) or banks[child][0] != base:
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
            if not 0 < base < len(banks) or banks[base][0] != 0:
                raise ValueError('active parent bank is not a prepared-source overlay')
            for target, value in zip(keys[base], decoded[base]):
                bank_register[target] = value
        if parent_register != bank_register:
            raise ValueError('active re-entry did not retain its actual parent register')
        if stage:
            check_child(base, first, wire, 1, first_ok)
    closure = report['closure']
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
    return {**native_audit,**product_audit,**reader_audit,'source_bits': numeral(source).bit_length(), 'status': report['status'],
            'proper_factor': closure is not None, 'emitted_assignment_gate_checks': values_checked,
            'retained_wires': len(ops), 'prepared_register_recovered_from_trail': True,
            'joint_cells_checked': len(joint_cells),
            'register_banks_checked': len(banks), 'nested_fuses_checked': len(report.get('nested_receipts', [])),
            'nested_factor_bit_contractions_checked': factor_count,
            'branch_continuation_checked': True, 'carried_native_bridge_image_checked': True,
            'scope': 'assignment and continuation audit',
            'return_sha256': file_digest(directory/'return.json')}


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('directory', type=Path)
    p.add_argument('output', type=Path)
    args = p.parse_args()
    result = verify(args.directory)
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result))
