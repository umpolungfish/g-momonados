#!/usr/bin/env python3
"""Execute an already lifted resident membrane through the Vox machine."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
import select
import sys

ROOT=Path(__file__).resolve().parents[2]


def digest(path):
    with path.open('rb') as stream:return hashlib.file_digest(stream,'sha256').hexdigest()


def main(directory,seconds,vox,output=None):
    module=directory/'membrane.imasm'
    if not module.exists():raise ValueError('lift the bound executable first')
    output=output.resolve() if output is not None else directory/'vox_execution'
    output.mkdir(exist_ok=False)
    (output/'evidence').mkdir(exist_ok=True)
    source_binding=directory/'binding.json'
    if not source_binding.exists():source_binding=directory/'launch_binding.json'
    bindings={'source_executable_sha256':digest(directory/'membrane'),'module_sha256':digest(module),
              'vox_sha256':digest(vox),'source_binding':str(source_binding.resolve()),
              'module':str(module.resolve()),'vox':str(vox.resolve()),
              'wall_boundary_seconds':seconds or None,'evidence_directory':str((output/'evidence').resolve())}
    (output/'active_execution.json').write_text(json.dumps(bindings,indent=2)+'\n')
    started=time.monotonic();first=None;timed_out=False
    with (output/'return.json').open('wb') as stdout,(output/'stderr').open('wb') as stderr:
        environment=dict(os.environ,VOX_STEPS='0',RESIDENT_EVIDENCE_DIR=str((output/'evidence').resolve()))
        for name in ['VOX_TRACE','VOX_LOADING','VOX_RANGE','VOX_WMEM','VOX_WMEM_RANGE']:
            environment.pop(name,None)
        with subprocess.Popen([str(vox),'run',str(module)],stdout=subprocess.PIPE,stderr=stderr,env=environment) as child:
            bindings.update(pid=child.pid)
            (output/'active_execution.json').write_text(json.dumps(bindings,indent=2)+'\n')
            while True:
                remaining=seconds-(time.monotonic()-started) if seconds and not timed_out else None
                if remaining is not None and remaining<=0:
                    timed_out=True;child.terminate();remaining=None
                ready,_,_=select.select([child.stdout],[],[],remaining)
                if not ready:continue
                block=os.read(child.stdout.fileno(),1024*1024)
                if not block:break
                if first is None:first=time.monotonic()-started
                stdout.write(block)
            status=child.wait()
    result={'source_executable_sha256':bindings['source_executable_sha256'],'module_sha256':bindings['module_sha256'],
            'vox_sha256':bindings['vox_sha256'],'elapsed_seconds':time.monotonic()-started,
            'wall_boundary_seconds':seconds,'evidence_directory':str((output/'evidence').resolve()),'timed_out':timed_out,'exit_code':status,
            'terminal_bytes':(output/'return.json').stat().st_size,
            'first_stdout_byte_seconds':first,'stderr_sha256':digest(output/'stderr'),
            'bindings_unchanged':bindings['source_executable_sha256']==digest(directory/'membrane')
                and bindings['module_sha256']==digest(module) and bindings['vox_sha256']==digest(vox)}
    (output/'receipt.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result))
    if status==0 and not timed_out and result['bindings_unchanged']:
        for name in ['source.imasm','membrane','relation.ops','register.cells','relation.cells','cell.rows',
                     'native_arithmetic.json','correlated.cells','correlated.topology','correlated_prepared.json',
                     'product.prepared.json','reader.index','prepared.json','bridge_provenance.json','parent_return.json']:
            if (directory/name).exists():(output/name).symlink_to((directory/name).resolve())
        binding_path=directory/'binding.json'
        if not binding_path.exists():binding_path=directory/'launch_binding.json'
        binding=json.loads(binding_path.read_text())
        report=json.loads((output/'return.json').read_text())
        binding.update(return_sha256=digest(output/'return.json'),first_stdout_byte_seconds=first,
                       process_lifetime_seconds=result['elapsed_seconds'],terminal_record_bytes=result['terminal_bytes'],
                       status=report['status'],closure=report['closure'],execution_substrate='Vox IMASM VM',
                       module_sha256=bindings['module_sha256'],vox_sha256=bindings['vox_sha256'])
        (output/'binding.json').write_text(json.dumps(binding,indent=2)+'\n')
        from resident_factor_return import factor_return
        checked=factor_return(report,(directory/'source.imasm').read_text().strip())
        checked['return_sha256']=digest(output/'return.json')
        (output/'factor_verification.json').write_text(json.dumps(checked,indent=2)+'\n')


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('directory',type=Path);p.add_argument('--seconds',type=int,default=0)
    p.add_argument('--vox',type=Path,default=ROOT/'Vox/target/x86_64-unknown-linux-gnu/release/vox')
    p.add_argument('--output',type=Path,help='fresh execution directory; preserves earlier VM returns')
    a=p.parse_args();main(a.directory,a.seconds,a.vox.resolve(),a.output)
