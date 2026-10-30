#!/usr/bin/env python3
"""Continue a certified locked checkpoint, then lift its verified return to Vox."""
import argparse
import fcntl
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import time

ROOT=Path(__file__).resolve().parents[2]
TOOLS=Path(__file__).resolve().parent


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream,'sha256').hexdigest()


def certified_parent(parent,source):
    certificate=json.loads((parent/'verification.json').read_text())
    if certificate['return_sha256']!=digest(parent/'return.json'):
        raise ValueError('checkpoint certificate binds another return')
    report=json.loads((parent/'return.json').read_text())
    if report['source_word']!=source.read_text().strip() or report['status']!='held':
        raise ValueError('checkpoint must retain the locked open source')
    return certificate


def process_identity(pid):
    try:
        fields=Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split()
    except FileNotFoundError:return None
    return fields[19],fields[0]


def main(parent,base,pid):
    parent=parent.resolve();base=base.resolve()
    source=ROOT/'G-mOMonadOS/measurements/resident_imasm_integration_20261012/inputs/fresh_balanced_512.imasm'
    state_path=base/'continuation_handoff.json'
    with (base/'continuation_handoff.lock').open('a+b') as guard:
        fcntl.flock(guard,fcntl.LOCK_EX|fcntl.LOCK_NB)
        if (base/'locked512').exists():raise ValueError('locked continuation output already exists')
        identity=process_identity(pid)
        if identity is None:raise ValueError('checkpoint auditor is no longer present')
        command=(Path(f'/proc/{pid}/cmdline').read_bytes().replace(b'\0',b' ').decode())
        if 'verify_relational_imasm_bridge.py' not in command or str(parent.relative_to(ROOT)) not in command:
            raise ValueError('observed process is not this checkpoint auditor')
        # Freeze the implementation expected at handoff. The baker retains
        # its own complete source and executable bindings on actual launch.
        sources=list(TOOLS.glob('resident_*.rs'))+list(TOOLS.glob('relational_*.rs'))
        sources += [TOOLS/n for n in ['continue_verified_resident.py','run_resident_dialect_integration.py',
                     'bake_relational_imasm_bridge.py','resident_embedded_archives.py',
                     'execute_resident_vox.py','compile_native_arithmetic_bridge.py']]
        hashes={str(p):digest(p) for p in sources}
        state={'status':'waiting_for_checkpoint_certificate','auditor_pid':pid,
               'auditor_start_ticks':identity[0],
               'auditor_command':command,'parent_return':str(parent/'return.json'),
               'source_sha256':digest(source),'handoff_sources':hashes,'started_unix':time.time()}
        def save():state_path.write_text(json.dumps(state,indent=2)+'\n')
        save()
        while True:
            current=process_identity(pid)
            if current is None or current[0]!=identity[0] or current[1]=='Z':break
            time.sleep(30)
        try:
            certified_parent(parent,source)
            if digest(source)!=state['source_sha256'] or any(digest(Path(p))!=h for p,h in hashes.items()):
                raise ValueError('handoff source changed while certification was running')
            state['status']='compiling_and_executing_locked_native';save()
            subprocess.run([sys.executable,str(TOOLS/'run_resident_dialect_integration.py'),'locked',
                            '--output',str(base),'--parent-return',str(parent/'return.json'),
                            '--source-product-return','--nested-reentry'],check=True)
            out=base/'locked512'
            native=json.loads((out/'verification.json').read_text())
            if not native['proper_factor'] or not native['full_word_execution']:
                raise ValueError('native return did not close the complete factor word')
            vox=ROOT/'Vox/target/x86_64-unknown-linux-gnu/release/vox'
            state.update(status='lifting_verified_locked_executable',vox_sha256=digest(vox));save()
            subprocess.run([str(vox),'imasm',str(out/'membrane'),'--output',str(out/'membrane.imasm')],check=True)
            state['status']='executing_locked_vox';save()
            subprocess.run([sys.executable,str(TOOLS/'execute_resident_vox.py'),str(out)],check=True)
            result=json.loads((out/'vox_execution/verification.json').read_text())
            if not result['proper_factor'] or not result['full_word_execution']:
                raise ValueError('Vox return did not close the complete factor word')
            state['status']='native_and_vox_returns_verified';save()
        except Exception as error:
            state.update(status='handoff_stopped',error=f'{type(error).__name__}: {error}');save()
            raise


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('parent',type=Path)
    parser.add_argument('base',type=Path)
    parser.add_argument('--auditor-pid',type=int,required=True)
    args=parser.parse_args();main(args.parent,args.base,args.auditor_pid)
