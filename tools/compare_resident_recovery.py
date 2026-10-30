#!/usr/bin/env python3
"""Compare a saved continuation with the resumed executable's initial checkpoint."""
import argparse
import hashlib
import json
from pathlib import Path


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def normalized(value):
    if isinstance(value, dict):
        return {key: normalized(child) for key, child in value.items()
                if key not in ('external_archive', 'trigger_to_readout_seconds')}
    if isinstance(value, list):
        return [normalized(child) for child in value]
    return value


def compare(parent, resumed):
    checkpoint = resumed / 'evidence/checkpoint.json'
    before = json.loads(parent.read_text())
    after = json.loads(checkpoint.read_text())
    introduced=[];introduced_empty_archives=set();prefix=None
    if 'preparation_word_control' not in before and 'preparation_word_control' in after:
        recovered=json.loads((resumed/'preparation_recovery.json').read_text())
        if recovered['parent_return_sha256']!=digest(parent) or recovered['prepared_control_sha256']!=digest(resumed/'prepared.json'):
            raise ValueError('prefix recovery binds another parent or compiled preparation')
        expected={'relation.ops','register.cells','relation.cells','cell.rows','reader.index'}
        if (resumed/'correlated.topology').exists():expected|={'correlated.cells','correlated.topology'}
        if {row['name'] for row in recovered['baseline_files']}!=expected:
            raise ValueError('prefix recovery omitted a retained baseline component')
        for row in recovered['baseline_files']:
            if Path(row['parent_file'])!=(parent.parent/row['name']).resolve() or Path(row['recovered_file'])!=(resumed/'recovered_preparation'/row['name']).resolve():
                raise ValueError('prefix recovery compared a different baseline path')
            if digest(Path(row['parent_file']))!=row['sha256'] or digest(Path(row['recovered_file']))!=row['sha256']:
                raise ValueError('recovered prefix changed a retained baseline component')
        prefix=json.loads((resumed/'prepared.json').read_text())['control_prefix']
        if after['preparation_word_control']!=prefix:
            raise ValueError('initial checkpoint lost its independently recovered prefix')
        before['preparation_word_control']=prefix
        introduced.append('preparation_word_control')
    if before.get('control_word') is None and after.get('control_word') is not None:
        if prefix is None:raise ValueError('legacy control recovery lacks its checked preparation prefix')
        active=before['continuation'].get('nested_frame') is not None
        expected={
            'control_word':{'word':prefix['word'],'dialect_pc':12,'completed_dialect_blocks':0,
                            'implemented_range_zero_based':[12,18],'full_word_execution':False},
            'inner_word_control':{'pc':19 if active else 20,'consistent':True,'block':0,
                                  'implemented_ranges_zero_based':[[19,26],[29,36]]},
            'outer_word_control':{'pc':19 if active else 18,'completed_interblock_seams':0,
                                  'implemented_positions_zero_based':[18,26,27,28]},
            'holding_frame':prefix['holding_frame'],
            'legacy_pending_work':{'phase':before['bridge_phase'],'action':'propagate_source_product'},
        }
        for name,value in expected.items():
            if before.get(name) is None:
                if after.get(name)!=value:raise ValueError('legacy recovery invented control state: '+name)
                before[name]=value;introduced.append(name)
        if 'reversal_receipts' not in before:
            empty=after.get('reversal_receipts')
            if not isinstance(empty,dict) or {key:empty.get(key) for key in ('records','bytes','encoding')}!={'records':0,'bytes':0,'encoding':'length-u64le-gzip-json'}:
                raise ValueError('legacy recovery introduced nonempty reversal history')
            if Path(empty['external_archive'])!=(resumed/'evidence/reversal.records').resolve():
                raise ValueError('legacy recovery introduced an unbound reversal archive')
            before['reversal_receipts']=empty;introduced.append('reversal_receipts')
            introduced_empty_archives.add('reversal_receipts')
    if normalized(before) != normalized(after):
        changed = sorted(key for key in before.keys() | after.keys()
                         if key != 'trigger_to_readout_seconds'
                         and normalized(before.get(key)) != normalized(after.get(key)))
        raise ValueError('recovery changed retained state: ' + ', '.join(changed))
    manifest = json.loads((resumed / 'embedded_archives.json').read_text())
    for row in manifest['prefixes']:
        if digest(Path(row['embedded_file'])) != row['sha256']:
            raise ValueError('compiled prefix changed: ' + row['embedded_file'])
    prefixes = {(row['source_archive'], row['bytes']): row for row in manifest['prefixes']}
    migrated = 0
    def check_archives(old, new):
        nonlocal migrated
        if isinstance(old, dict):
            if 'external_archive' in old:
                row = prefixes[(old['external_archive'], old['bytes'])]
                remaining = old['bytes']
                hashed = hashlib.sha256()
                with Path(new['external_archive']).open('rb') as stream:
                    while remaining:
                        block = stream.read(min(1024 * 1024, remaining))
                        if not block:
                            raise ValueError('migrated archive is shorter than its retained prefix')
                        hashed.update(block)
                        remaining -= len(block)
                if hashed.hexdigest() != row['sha256']:
                    raise ValueError('execution changed the compiled archive prefix')
                migrated += 1
            else:
                for key, child in old.items():
                    if key not in introduced_empty_archives:check_archives(child, new[key])
        elif isinstance(old, list):
            for child, restored in zip(old, new):
                check_archives(child, restored)
    check_archives(before, after)
    receipt = dict(passed=True, all_state_fields_equal_except_execution_clock_and_archive_paths=not introduced,
                   all_preexisting_state_fields_equal_except_execution_clock_and_archive_paths=True,
                   introduced_compiled_evidence_fields=introduced,
                   parent_sha256=digest(parent), initial_checkpoint_sha256=digest(checkpoint),
                   embedded_prefixes=len(manifest['prefixes']), migrated_prefixes_checked=migrated,
                   holding_frame=after.get('holding_frame'), control_word=after.get('control_word'),
                   instrument_sha256=digest(Path(__file__)))
    (resumed / 'recovery_comparison.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('parent', type=Path)
    parser.add_argument('resumed', type=Path)
    args = parser.parse_args()
    compare(args.parent, args.resumed)
