#!/usr/bin/env python3
"""Resolve Vox native address samples against the exact profiled ELF."""
import argparse
import bisect
from collections import Counter
import hashlib
import json
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('binary', type=Path)
parser.add_argument('prefix', type=Path)
args = parser.parse_args()
symbols = []
raw = subprocess.run(['nm', '-S', '--defined-only', '--numeric-sort', str(args.binary)],
                     check=True, capture_output=True, text=True).stdout
for line in raw.splitlines():
    fields = line.split()
    if len(fields) == 4 and fields[2] in 'TtWw':
        symbols.append((int(fields[0], 16), int(fields[1], 16), fields[3]))
symbols.sort()
starts = [symbol[0] for symbol in symbols]
counts = Counter()
sample_times = []
samples = Path(str(args.prefix) + '.samples.tsv')
for line in samples.read_text().splitlines()[1:]:
    fields = line.split('\t')
    if len(fields) != 2:
        continue
    try:
        timestamp, address = float(fields[0]), int(fields[1], 16)
    except ValueError:
        continue  # A bounded interrupted observation can end in a partial row.
    sample_times.append(timestamp)
    index = bisect.bisect_right(starts, address) - 1
    name = (symbols[index][2] if index >= 0 and address < symbols[index][0] + symbols[index][1]
            else '[external or unassigned]')
    counts[name] += 1
assert sample_times, 'native profile contains no complete address samples'
names = [name for name, _ in counts.most_common()]
demangled = subprocess.run(['c++filt'], input='\n'.join(names) + '\n', check=True,
                           capture_output=True, text=True).stdout.splitlines()
report = {'sample_count': sum(counts.values()), 'last_sample_seconds': max(sample_times),
          'binary_sha256': hashlib.sha256(args.binary.read_bytes()).hexdigest(),
          'samples_sha256': hashlib.sha256(samples.read_bytes()).hexdigest(),
          'scope': 'captured address samples; not hardware cycle measurements',
          'symbols': [{'samples': counts[raw], 'percent': 100 * counts[raw] / sum(counts.values()),
                       'symbol': name} for raw, name in zip(names, demangled)]}
Path(str(args.prefix) + '_summary.json').write_text(json.dumps(report, indent=2) + '\n')
for entry in report['symbols'][:10]:
    print(f"{entry['percent']:.3f}% {entry['symbol']}")
