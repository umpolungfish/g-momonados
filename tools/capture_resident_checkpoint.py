#!/usr/bin/env python3
"""Capture and independently certify an internal checkpoint without stopping production."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
from verify_relational_imasm_bridge import verify


def digest(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def capture(directory,output):
    directory=directory.resolve();output=output.resolve();output.mkdir(exist_ok=False)
    # Atomic rename gives this open/read a complete checkpoint generation.
    with (directory/'evidence/checkpoint.json').open('rb') as stream:
        with (output/'return.json').open('wb') as saved:shutil.copyfileobj(stream,saved)
    binding=json.loads((directory/'launch_binding.json').read_text())
    for name in ['source.imasm','membrane','relation.ops','register.cells','relation.cells','cell.rows',
                 'native_arithmetic.json','correlated.cells','correlated.topology','correlated_prepared.json',
                 'product.prepared.json','reader.index','prepared.json','bridge_provenance.json','parent_return.json']:
        source=directory/name
        if source.exists():(output/name).symlink_to(source)
    binding.update(return_sha256=digest(output/'return.json'),readout_kind='internal_checkpoint',
                   first_stdout_byte_seconds=None,terminal_record_bytes=0)
    (output/'binding.json').write_text(json.dumps(binding,indent=2)+'\n')
    result=verify(output)
    (output/'verification.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result))


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('directory',type=Path);parser.add_argument('output',type=Path)
    args=parser.parse_args();capture(args.directory,args.output)
