#!/usr/bin/env python3
"""Supervise a baked membrane and request its retained return at a run boundary."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import time

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('executable',type=Path)
parser.add_argument('output_prefix',type=Path)
parser.add_argument('--seconds',type=float,default=300)
args=parser.parse_args()
binary=args.executable.resolve();prefix=args.output_prefix.resolve()
stdout=prefix.with_suffix('.json');stderr=prefix.with_suffix('.stderr')
if any(p.exists() for p in (stdout,stderr,prefix.with_suffix('.supervision.json'))):
    raise SystemExit('preserve existing receipts: choose a new output prefix')
start=time.monotonic();requested=False;forced=False;bytes_at_request=None
with stdout.open('wb') as out,stderr.open('wb') as err:
    child=subprocess.Popen([str(binary)],stdout=out,stderr=err,start_new_session=True)
    try:
        code=child.wait(timeout=args.seconds)
    except subprocess.TimeoutExpired:
        requested=True;bytes_at_request=stdout.stat().st_size
        os.kill(child.pid,signal.SIGUSR1)
        # The request ends transport. Readout and complete-state serialization
        # finish normally, including dynamically retained witness banks.
        code=child.wait()
receipt={'binary':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),
         'external_execution_boundary_seconds':args.seconds,'held_return_requested':requested,
         'bytes_before_return_request':bytes_at_request,'forced_termination':forced,
         'exit_code':code,'driver_seconds':time.monotonic()-start,'stdout':str(stdout)}
prefix.with_suffix('.supervision.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt,indent=2))
raise SystemExit(0 if code in (0,2) and not forced else 1)
