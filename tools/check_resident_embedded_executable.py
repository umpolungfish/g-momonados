#!/usr/bin/env python3
"""Compile archive prefixes and recover them with both input paths absent."""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
from resident_embedded_archives import embed_prefixes


def main(output):
    output = output.resolve()
    (output / 'src').mkdir(parents=True, exist_ok=False)
    values = [(0, []), (0, [3, 2, 1])]
    prefix = b''
    for value in values:
        record = gzip.compress(json.dumps(value).encode())
        prefix += struct.pack('<Q', len(record)) + record
    parent = output / 'parent.records'
    parent.write_bytes(prefix + b'unbound later records')
    reference = dict(external_archive=str(parent), bytes=len(prefix), records=2,
                     encoding='length-u64le-gzip-json')
    declaration, manifest = embed_prefixes(output, [reference])
    module = Path(__file__).with_name('resident_evidence_store.rs').resolve()
    source = f'''#[path={json.dumps(str(module))}] mod evidence;
{declaration}
fn main() {{
 let reference:serde_json::Value=serde_json::from_str({json.dumps(json.dumps(reference))}).unwrap();
 let mut archive=evidence::Archive::<(usize,Vec<u8>)>::load_bound(&reference,EMBEDDED_ARCHIVES);
 assert_eq!(archive.len(),2); assert_eq!(archive.get(1),(0,vec![3,2,1]));
 archive.push((1,vec![8]));
 archive.migrate(std::path::Path::new("returned.records")); archive.sync();
 assert_eq!(archive.get(2),(1,vec![8]));
 println!("{{}}",archive.view());
}}
'''
    (output / 'src/main.rs').write_text(source)
    (output / 'Cargo.toml').write_text('''[package]
name="resident-embedded-executable-instrument"
version="0.1.0"
edition="2021"
[dependencies]
serde_json="1"
serde="1"
flate2="1"
''')
    env = dict(os.environ, CARGO_TARGET_DIR=str(output.parent / 'embedded_executable_cargo'))
    with (output / 'build.log').open('wb') as log:
        subprocess.run(['cargo', 'build', '--offline', '--release', '--target', 'x86_64-unknown-linux-gnu', '--manifest-path',
                        str(output / 'Cargo.toml')], env=env, stdout=log,
                       stderr=subprocess.STDOUT, check=True)
    executable = Path(env['CARGO_TARGET_DIR']) / 'x86_64-unknown-linux-gnu/release/resident-embedded-executable-instrument'
    parent.rename(parent.with_suffix('.retained'))
    embedded = Path(manifest['prefixes'][0]['embedded_file'])
    embedded.rename(embedded.with_suffix('.retained'))
    run = output / 'isolated'
    run.mkdir()
    result = subprocess.run([str(executable)], cwd=run, capture_output=True, check=True)
    returned = json.loads(result.stdout)
    raw = (run / 'returned.records').read_bytes()
    assert raw[:len(prefix)] == prefix
    assert returned['records'] == 3
    receipt = dict(passed=True, original_path_absent=not parent.exists(),
                   compiled_input_path_absent=not embedded.exists(),
                   returned_prefix_sha256=hashlib.sha256(raw[:len(prefix)]).hexdigest(),
                   executable_sha256=hashlib.file_digest(executable.open('rb'), 'sha256').hexdigest(),
                   source_sha256=hashlib.sha256(module.read_bytes()).hexdigest(),
                   returned=returned)
    (output / 'receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    print(json.dumps(receipt))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    main(parser.parse_args().output)
