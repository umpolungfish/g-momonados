#!/usr/bin/env python3
"""Execute an already lifted resident membrane through the Vox machine."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

ROOT=Path(__file__).resolve().parents[2]


def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()


def main(directory,seconds,vox):
    module=directory/'membrane.imasm'
    if not module.exists():raise ValueError('lift the bound executable first')
    output=directory/'vox_execution';output.mkdir(exist_ok=False)
    started=time.monotonic();first=None;timed_out=False
    with (output/'return.json').open('wb') as stdout,(output/'stderr').open('wb') as stderr:
        environment=dict(os.environ,VOX_STEPS='0')
        for name in ['VOX_TRACE','VOX_LOADING','VOX_RANGE','VOX_WMEM','VOX_WMEM_RANGE']:
            environment.pop(name,None)
        with subprocess.Popen([str(vox),'run',str(module)],stdout=stdout,stderr=stderr,env=environment) as child:
            try:status=child.wait(timeout=seconds if seconds else None)
            except subprocess.TimeoutExpired:
                timed_out=True;child.terminate();status=child.wait()
    result={'source_executable_sha256':digest(directory/'membrane'),'module_sha256':digest(module),
            'vox_sha256':digest(vox),'elapsed_seconds':time.monotonic()-started,
            'wall_boundary_seconds':seconds,'timed_out':timed_out,'exit_code':status,
            'terminal_bytes':(output/'return.json').stat().st_size,
            'stderr_sha256':digest(output/'stderr')}
    (output/'receipt.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('directory',type=Path);p.add_argument('--seconds',type=int,default=0)
    p.add_argument('--vox',type=Path,default=ROOT/'Vox/target/x86_64-unknown-linux-gnu/release/vox')
    a=p.parse_args();main(a.directory,a.seconds,a.vox.resolve())
