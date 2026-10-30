"""Bind retained archive prefixes into the static continuation at compilation."""
import hashlib
import json
from pathlib import Path
import struct


def _references(value):
    if isinstance(value, dict):
        if 'external_archive' in value:
            if value.get('encoding') != 'length-u64le-gzip-json':
                raise ValueError('unsupported retained archive encoding')
            yield value
        else:
            for child in value.values():
                yield from _references(child)
    elif isinstance(value, list):
        for child in value:
            yield from _references(child)


def embed_prefixes(output, values, reuse=False):
    """Copy exact certified boundaries; later appends cannot enter the image."""
    output = Path(output)
    directory = output / 'embedded_archives'
    directory.mkdir(exist_ok=reuse)
    references = {}
    for value in values:
        for ref in _references(value):
            key = (ref['external_archive'], ref['bytes'])
            if key in references and references[key]['records'] != ref['records']:
                raise ValueError('one archive prefix has conflicting record counts')
            references[key] = ref
    bindings = []
    rows = []
    prior=json.loads((output/'embedded_archives.json').read_text()) if reuse else None
    for index, ((source, boundary), ref) in enumerate(sorted(references.items())):
        if type(boundary) is not int or boundary < 0 or type(ref['records']) is not int or ref['records'] < 0:
            raise ValueError('archive boundaries and counts must be nonnegative integers')
        target = directory / f'{index:03d}.records'
        if reuse:
            row=prior['prefixes'][index]
            if (row['source_archive'],row['bytes'],row['records'],row['embedded_file'])!=(source,boundary,ref['records'],str(target.resolve())):
                raise ValueError('prepared archive manifest differs from the retained checkpoint')
            with target.open('rb') as stream:actual=hashlib.file_digest(stream,'sha256').hexdigest()
            if target.stat().st_size!=boundary or actual!=row['sha256']:
                raise ValueError('copied archive prefix changed before static compilation')
            rows.append(row)
            continue
        digest = hashlib.sha256()
        offset = 0
        with Path(source).open('rb') as parent, target.open('xb') as saved:
            for _ in range(ref['records']):
                header = parent.read(8)
                if len(header) != 8:
                    raise ValueError('truncated archive header')
                length, = struct.unpack('<Q', header)
                offset += 8 + length
                if offset > boundary:
                    raise ValueError('record crosses the certified archive boundary')
                digest.update(header)
                saved.write(header)
                remaining = length
                while remaining:
                    block = parent.read(min(1024 * 1024, remaining))
                    if not block:
                        raise ValueError('truncated archive record')
                    saved.write(block)
                    digest.update(block)
                    remaining -= len(block)
            if offset != boundary:
                raise ValueError('archive record count does not cover its boundary')
        rows.append({'source_archive': source, 'bytes': boundary, 'records': ref['records'],
                     'embedded_file': str(target.resolve()), 'sha256': digest.hexdigest()})
    manifest = {'format': 'resident-embedded-archives-v1', 'prefixes': rows}
    if prior is not None and len(prior['prefixes'])!=len(rows):
        raise ValueError('prepared archive manifest has extra prefixes')
    (output / 'embedded_archives.json').write_text(json.dumps(manifest, indent=2) + '\n')
    # Keep retained bytes outside rustc's constant evaluator and LLVM IR. Place
    # this read-only section after ordinary code/data so multi-gigabyte prefixes
    # do not separate normal small-model code from its globals or unwind data.
    assembly=['.section .resident_archives,"a",@progbits']
    declarations=[]
    for index,row in sorted(enumerate(rows),key=lambda entry:entry[1]['bytes']):
        symbol=f'resident_archive_{index}'
        assembly += [f'.global {symbol}',f'.hidden {symbol}',f'{symbol}:',
                     '.incbin '+json.dumps(row['embedded_file'])]
        declarations.append(f'static {symbol}: u8;')
        bindings.append(f'({json.dumps(row["source_archive"])}, unsafe {{std::slice::from_raw_parts(std::ptr::addr_of!({symbol}), {row["bytes"]})}})')
    assembly.append('.section .note.GNU-stack,"",@progbits')
    assembly_text='\n'.join(assembly)+'\n'
    assembly_path=output/'resident_archives.S'
    if not assembly_path.exists() or assembly_path.read_text()!=assembly_text:assembly_path.write_text(assembly_text)
    (output/'resident_archives.ld').write_text('SECTIONS { _end = ADDR(__libc_freeres_ptrs) + SIZEOF(__libc_freeres_ptrs); .resident_archives ALIGN(4096) : { KEEP(*(.resident_archives)) } } INSERT AFTER __libc_freeres_ptrs;\n')
    (output/'build.rs').write_text('''fn main() {
    let directory=std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let object=std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("resident_archives.o");
    let assembly=directory.join("resident_archives.S");
    if !object.exists() || std::fs::metadata(&object).unwrap().modified().unwrap() < std::fs::metadata(&assembly).unwrap().modified().unwrap() {
    let status=std::process::Command::new("cc").arg("-c").arg(directory.join("resident_archives.S")).arg("-o").arg(&object).status().unwrap();
    assert!(status.success(),"retained archive assembly failed");
    }
    println!("cargo:rustc-link-arg={}",object.display());
    println!("cargo:rustc-link-arg=-Wl,-T,{}",directory.join("resident_archives.ld").display());
    println!("cargo:rerun-if-changed=resident_archives.S");
    println!("cargo:rerun-if-changed=resident_archives.ld");
    println!("cargo:rerun-if-changed=embedded_archives.json");
}
''')
    return 'unsafe extern "C" {\n'+'\n'.join(declarations)+'\n}\n'+'fn embedded_archives() -> Vec<(&\'static str, &\'static [u8])> { vec![\n' + ',\n'.join(bindings) + '\n] }\n', manifest
