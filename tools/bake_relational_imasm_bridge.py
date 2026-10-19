#!/usr/bin/env python3
"""Bake N-only gate relations and native bridge images into a silent executable."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import time
import os

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def bridges(tables_path, receipt_path):
    tables = json.loads(tables_path.read_text())
    receipt = json.loads(receipt_path.read_text())
    # These cached finite transitions are compiled from actual typed graphs,
    # not invented glyph meanings. Refuse stale source bindings.
    for path, expected in receipt['core_sources'].items():
        if digest(Path(path)) != expected:
            raise ValueError(f'bridge core changed since native execution: {path}')
    programs = {}
    for program in receipt['programs']:
        if digest(Path(program['source'])) != program['sha256']:
            raise ValueError('primitive program changed since native execution')
        programs[program['name']] = program
    glyph_source = (ROOT / 'G-mOMonadOS/src/catalog.rs').read_text()
    glyph_source = glyph_source.split('pub fn primitive_glyph(')[1].split('\npub fn ')[0]
    names = {glyph: name.rstrip('_') for name, glyph in re.findall(
        r'IgPrim::(\w+)\s*=>\s*"([^"]+)"', glyph_source)}
    # The supplied current appendix's supercritical program is digital/ha;
    # the digital directory also retains two older spelling aliases.
    names['𐑣'] = 'ha'
    relation_source = ROOT / ('ob3ect/digital/semiprime_factor_pair_g_del_encoding_relationshi_4fe32570/'
                              'semiprime_factor_pair_g_del_encoding_relationshi_4fe32570_ob3ect.json')
    relation = json.loads(relation_source.read_text())
    glyphs = [g for g in relation['grounded_tuple'] if g not in '⟨⟩·; ,']
    if len(glyphs) != 12:
        raise ValueError('retained relation descriptor must have twelve axes')
    anchor = [next(i for i, v in enumerate(axis) if v['glyph'] == glyph)
              for axis, glyph in zip(tables['axis_values'], glyphs)]
    current = anchor[:]
    hops = []
    ancestry = []
    # Successive environments operate on the tuple returned by the last one.
    # None of these intermediate hops projects back into the canonical dialect.
    for dialect, operation in [(20, 2), (24, 0), (27, 1)]:
        before = current[:]
        conflict = []
        for axis, values in enumerate(tables['axis_values']):
            cell = tables['tables'][dialect]['axis_operations'][operation][axis][
                current[axis] * len(values) + anchor[axis]]
            current[axis] = cell[0]
            conflict.append(bool(cell[1]))
        image = list(range(16))
        used = []
        for axis, coordinate in enumerate(current):
            glyph = tables['axis_values'][axis][coordinate]['glyph']
            program = programs[names[glyph]]
            action = [row['terminal'] for row in program['executions']]
            image = [action[value] for value in image]
            used.append({'axis': axis, 'glyph': glyph, 'type': program['name'],
                         'source': program['source'], 'sha256': program['sha256']})
        hops.append(image)
        ancestry.append({'dialect': dialect, 'operation': tables['operation_names'][operation],
                         'tuple_before': before, 'tuple_after': current[:],
                         'conflicts': conflict, 'type_programs': used, 'native_hop_image': image})
    return hops, {'relation_descriptor': str(relation_source),
                  'relation_descriptor_sha256': digest(relation_source),
                  'tables_path': str(tables_path.resolve()),
                  'native_execution_receipt_path': str(receipt_path.resolve()),
                  'tables_sha256': digest(tables_path), 'native_execution_receipt_sha256': digest(receipt_path),
                  'source_boundary': 'descriptor and native tables are source-independent; N binds the product circuit',
                  'ancestry': ancestry}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('source', type=Path)
    p.add_argument('output', type=Path)
    p.add_argument('--tables', required=True, type=Path)
    p.add_argument('--native-bridges', required=True, type=Path)
    p.add_argument('--seconds', type=int, default=0, help='0 runs until factor production or external termination')
    p.add_argument('--resume', type=Path)
    p.add_argument('--joint-cells', action='store_true')
    p.add_argument('--nested-reentry', action='store_true')
    p.add_argument('--resident-dialects', action='store_true')
    p.add_argument('--source-product-return', action='store_true')
    args = p.parse_args()
    if args.seconds < 0:
        raise ValueError('seconds must be nonnegative; 0 disables the production deadline')
    started = time.monotonic()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    source = args.source.read_text().strip()
    hops, provenance = bridges(args.tables, args.native_bridges)
    (out / 'source.imasm').write_text(source + '\n')
    (out / 'bridge_provenance.json').write_text(json.dumps(provenance, indent=2, ensure_ascii=False) + '\n')
    if args.resident_dialects:
        from compile_native_arithmetic_bridge import compile_bridge
        (out / 'native_arithmetic.json').write_text(json.dumps(
            compile_bridge(args.tables,args.native_bridges),indent=2)+'\n')
    rust_hops = '[' + ','.join('[' + ','.join(map(str, hop)) + ']' for hop in hops) + ']'
    imports = '#![allow(dead_code)]\nextern crate alloc;\n'
    modules = {'vox': ROOT / 'Vox/src/vox.rs',
               'fixed_point_imasm': ROOT / 'Vox/src/fixed_point_imasm.rs',
               'fixed_point_word_arithmetic': ROOT / 'Vox/src/fixed_point_word_arithmetic.rs',
               'factor_relation': ROOT / 'G-mOMonadOS/src/factor_relation.rs',
               'reentry_powerset': ROOT / 'G-mOMonadOS/src/reentry_powerset.rs',
               'bridge_engine': Path(__file__).with_name('relational_imasm_bridge.rs'),
               'joint_cells': Path(__file__).with_name('relational_joint_cells.rs')}
    for name, path in modules.items():
        imports += f'#[path={json.dumps(str(path))}] mod {name};\n'
    source_hashes = {str(path): digest(path) for path in [Path(__file__), *modules.values()]}
    for name in ['resident_dialect_relations.rs','resident_product_return.rs','resident_reader_index.rs','resident_evidence_store.rs','compile_native_arithmetic_bridge.py','relational_dialect.py']:
        path=Path(__file__).with_name(name)
        source_hashes[str(path)]=digest(path)
    snapshot=out/'source_snapshot'
    snapshot.mkdir()
    for index,path in enumerate(source_hashes):
        shutil.copyfile(path,snapshot/f'{index:02d}_{Path(path).name}')
    native_program='Some(include_str!('+json.dumps(str(out/'native_arithmetic.json'))+'))' if args.resident_dialects else 'None'
    imports += f'const HOPS:[[u8;16];{len(hops)}]={rust_hops};\n'
    imports += f'const SOURCE:&str=include_str!({json.dumps(str(out / "source.imasm"))});\n'
    (out / 'src').mkdir()
    main_path = out / 'src/main.rs'
    manifest = '''[package]
name="relational-imasm-bridge"
version="0.1.0"
edition="2021"
[dependencies]
num-bigint="0.4"
num-traits="0.2"
serde_json="1"
serde="1"
flate2="1"
[profile.release]
opt-level=3
'''
    (out / 'Cargo.toml').write_text(manifest)
    target = out.parent / 'relational_bridge_cargo'
    env = dict(__import__('os').environ, CARGO_TARGET_DIR=str(target),
               RUSTFLAGS='-C target-feature=+crt-static')
    command = ['cargo', 'build', '--offline', '--release', '--target', 'x86_64-unknown-linux-gnu',
               '--manifest-path', str(out / 'Cargo.toml')]
    binary = target / 'x86_64-unknown-linux-gnu/release/relational-imasm-bridge'
    resumed = 'None'
    if args.resume:
        parent = args.resume.resolve().parent
        report = json.loads(args.resume.read_text())
        certificate = json.loads((parent / 'verification.json').read_text())
        parent_binding = json.loads((parent / 'binding.json').read_text())
        if certificate['return_sha256'] != digest(args.resume):
            raise ValueError('resume return differs from its independent verification')
        for name, key in [('relation.ops', 'relation_sha256'),
                          ('register.cells', 'prepared_register_sha256')]:
            if digest(parent / name) != parent_binding[key]:
                raise ValueError('resume source relation/prepared bank changed')
        if report['source_word'] != source or report['status'] != 'held':
            raise ValueError('resume requires the same open N-only relation')
        if args.resident_dialects:
            if report.get('resident_dialect') is None:
                raise ValueError('resident dialect resumption needs a correlated parent')
            if (parent/'native_arithmetic.json').read_bytes() != (out/'native_arithmetic.json').read_bytes():
                raise ValueError('native arithmetic program changed at continuation')
            for name in ['correlated.cells','correlated.topology','correlated_prepared.json']:
                (out/name).symlink_to(parent/name)
        if args.source_product_return:
            if (parent/'product.prepared.json').exists():(out/'product.prepared.json').symlink_to(parent/'product.prepared.json')
            else:(out/'product.prepared.json').write_text('{"banks":[],"visits":0,"contractions":0}\n')
        if json.loads((parent / 'bridge_provenance.json').read_text()) != provenance:
            raise ValueError('resume bridge provenance differs')
        for name in ['relation.ops', 'register.cells', 'prepared.json']:
            (out / name).symlink_to(parent / name)
        (out / 'parent_return.json').symlink_to(args.resume.resolve())
        resumed = 'Some(include_str!(' + json.dumps(str(out / 'parent_return.json')) + '))'
        main_path.write_text(imports + 'fn main(){bridge_engine::compile_joint_bank(include_bytes!('
                             + json.dumps(str(out / 'relation.ops')) + '),std::path::Path::new('
                             + json.dumps(str(out)) + '),' + str(args.joint_cells).lower() + ');}\n')
        with (out / 'preparation_build.log').open('wb') as log:
            subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        subprocess.run([str(binary)], check=True, capture_output=True)
    else:
        main_path.write_text(imports + 'fn main(){bridge_engine::prepare_with_product(SOURCE,std::path::Path::new('
                             + json.dumps(str(out)) + '),HOPS.to_vec(),'
                             + str(args.joint_cells).lower() + ',' + native_program + ','+str(args.source_product_return).lower()+');}\n')
        with (out / 'preparation_build.log').open('wb') as log:
            subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        subprocess.run([str(binary)], check=True, capture_output=True)
    native_bank='Some((include_str!('+json.dumps(str(out/'native_arithmetic.json'))+'),include_bytes!('+json.dumps(str(out/'correlated.topology'))+') as &[u8],include_bytes!('+json.dumps(str(out/'correlated.cells'))+') as &[u8]))' if args.resident_dialects else 'None'
    product_bank='Some(include_str!('+json.dumps(str(out/'product.prepared.json'))+'))' if args.source_product_return else 'None'
    reader_index='Some(include_bytes!('+json.dumps(str(out/'reader.index'))+') as &[u8])'
    main_path.write_text(imports + 'fn main(){let report=bridge_engine::execute_archived(SOURCE,include_bytes!('
                         + json.dumps(str(out / 'relation.ops')) + '),include_bytes!('
                         + json.dumps(str(out / 'register.cells')) + f'),HOPS.to_vec(),{args.seconds},{resumed},include_bytes!('
                         + json.dumps(str(out / 'relation.cells')) + '),include_bytes!('
                         + json.dumps(str(out / 'cell.rows')) + '),'
                         + str(args.nested_reentry).lower() + ',' + native_bank + ','+product_bank+','+reader_index+',Some('+json.dumps(str(out/'evidence'))+'));'
                         + 'println!("{}",report);}\n')
    with (out / 'execution_build.log').open('wb') as log:
        subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    # Move the only built executable; keep Cargo's dependency cache shared.
    executable = out / 'membrane'
    shutil.move(binary, executable)
    compiled = time.monotonic() - started
    # Bind the compiled carrier before triggering it. A killed process still
    # leaves enough source/executable evidence to certify its saved checkpoint.
    launch_binding={
        'source_sha256':digest(out/'source.imasm'),'executable_sha256':digest(executable),
        'compilation_seconds':compiled,'relation_sha256':digest(out/'relation.ops'),
        'prepared_register_sha256':digest(out/'register.cells'),
        'joint_cells_sha256':digest(out/'relation.cells'),'native_cell_rows_sha256':digest(out/'cell.rows'),
        'correlated_artifacts':{name:digest(out/name) for name in
            (['native_arithmetic.json','correlated.cells','correlated.topology','correlated_prepared.json'] if args.resident_dialects else [])
            +(['product.prepared.json'] if args.source_product_return else [])+['reader.index']},
        'sources':source_hashes,'parent_return_sha256':digest(args.resume) if args.resume else None,
        'production_deadline_seconds':args.seconds or None}
    (out/'launch_binding.json').write_text(json.dumps(launch_binding,indent=2)+'\n')
    triggered=time.monotonic();first_byte=None;terminal_bytes=0
    with (out / 'return.json').open('wb') as log, (out / 'execution.stderr').open('wb') as err:
        with subprocess.Popen([str(executable)],stdout=subprocess.PIPE,stderr=err) as child:
            while True:
                block=os.read(child.stdout.fileno(),1024*1024)
                if not block:break
                if first_byte is None:first_byte=time.monotonic()-triggered
                log.write(block);terminal_bytes+=len(block)
            if child.wait()!=0:raise subprocess.CalledProcessError(child.returncode,[str(executable)])
    process_lifetime=time.monotonic()-triggered
    report = json.loads((out / 'return.json').read_text())
    if any(digest(Path(path)) != expected for path, expected in source_hashes.items()):
        raise ValueError('compiler/interpreter source changed during bake or execution')
    native_bindings={name:digest(out/name) for name in ['native_arithmetic.json','correlated.cells','correlated.topology','correlated_prepared.json']} if args.resident_dialects else {}
    if args.source_product_return:native_bindings['product.prepared.json']=digest(out/'product.prepared.json')
    native_bindings['reader.index']=digest(out/'reader.index')
    (out / 'binding.json').write_text(json.dumps({
        'correlated_artifacts':native_bindings,
        'source_sha256': digest(out / 'source.imasm'), 'executable_sha256': digest(executable),
        'return_sha256': digest(out / 'return.json'), 'compilation_seconds': compiled,
        'first_stdout_byte_seconds':first_byte,'process_lifetime_seconds':process_lifetime,
        'terminal_record_bytes':terminal_bytes,
        'relation_sha256': digest(out / 'relation.ops'), 'prepared_register_sha256': digest(out / 'register.cells'),
        'joint_cells_sha256': digest(out / 'relation.cells'), 'native_cell_rows_sha256': digest(out / 'cell.rows'),
        'sources': source_hashes, 'parent_return_sha256': digest(args.resume) if args.resume else None,
        'status': report['status'], 'closure': report['closure']}, indent=2) + '\n')
    print(json.dumps({'directory': str(out), 'status': report['status'], 'closure': report['closure'],
                      'contractions': report['contractions'], 'branches': report['branches']}))


if __name__ == '__main__':
    main()
