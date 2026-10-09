"""Retain the minimum-correction changed placements from complete-return transport."""
from pathlib import Path
import ast
import gzip
import re
import subprocess
import argparse

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--complete-return', type=int, choices=(0, 1), default=0,
                    help='Select the preserved complete-return transport seed.')
options = parser.parse_args()

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
logs = []


def load_functions(path, names):
    tree = ast.parse(path.read_text())
    nodes = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name in names]
    assert {n.name for n in nodes} == set(names)
    exec(compile(ast.Module(body=nodes, type_ignores=[]), str(path), 'exec'), globals())


load_functions(HERE/'residual_copy_completion.py', ['call', 'field', 'word', 'op', 'normalize'])
load_functions(HERE/'two_sided_partial_return.py', ['magnitude_word', 'certify'])
load_functions(HERE/'placement_full_return.py', ['frame'])
record = HERE/('operand_transport_combined_return_complete_'+str(options.complete_return)+'.md')
text = record.read_text()
rows = ast.literal_eval(re.search(r'Complete comparisons: (\[.*\])', text).group(1))
changed = [row for row in rows if row[-1]]
minimum = min(row[-2] for row in changed)
wanted = {tuple(row[:6]) for row in changed if row[-2] == minimum}
source = field(call('encode', (HERE/'source.txt').read_text().strip()), 'word')
previous = dict(ast.literal_eval(re.search(r'Final operand B: (\[.*\])', text).group(1)))
rounds = ast.literal_eval(re.search(r'Complete accepted paths: (\[.*\])', text).group(1))
if rounds:
    section = (HERE/'transport_placement_returns.md').read_text().split('Branch indexes: ')[1:][options.complete_return]
    numeral = re.search(r'^placement word: (.*)$', section, re.M).group(1)
    initial_b = {i:1 for i in range(len(numeral[1:-3])//5)
                 if numeral[1:-3][5*i:5*i+5] == FILLED}
else:
    initial_b = previous
seeds, placements = [], {}
with gzip.open(record.with_suffix('.log.gz'), 'rt') as archive:
    for line in archive:
        if not line.startswith('TRANSPORT PATH '):
            continue
        prefix = re.match(r'TRANSPORT PATH \((\d+), (\d+), (\d+), (-?\d+), (True|False), (True|False),', line)
        assert prefix
        key = tuple(int(x) for x in prefix.groups()[:4])+tuple(x=='True' for x in prefix.groups()[4:])
        if key not in wanted:
            continue
        row = ast.literal_eval(line[len('TRANSPORT PATH '):])
        A,B,R = (dict(row[i]) for i in (8,9,10))
        parent_b = initial_b if key[0] == 0 else dict(rounds[key[0]-1][4])
        assert normalize(B)[0] != normalize(parent_b)[0]
        assert sum(abs(c) for c in R.values()) == minimum
        certify(source, A, B, R)
        seeds.append((key, sorted(A.items()), sorted(B.items()), sorted(R.items())))
        sign, placement = magnitude_word(B)
        placements.setdefault(placement, []).append(key)
assert len(seeds) == len(wanted)
report = ['# Changed placements at minimum correction', '',
          'Parent: '+record.name+'.',
          f'Retained endpoints: {len(seeds)}; correction magnitude: {minimum}.',
          'Native Gödel verifies every retained complete signed equation.', '',
          'Complete seeds: '+repr(seeds), '']
for placement, keys in placements.items():
    division = frame(source, 'divmod', placement)
    quotient = field(call('encode', field(division, 'result')), 'word')
    remainder = field(call('encode', field(division, 'remainder')), 'word')
    assert op(op(placement, 'mul', quotient), 'add', remainder) == source
    exact = remainder == word([])
    report.extend(['Placement keys: '+repr(keys)+'.',
                   'Exact placement return: '+str(exact)+'.'])
    for label,numeral in [('placement',placement),('quotient',quotient),('remainder',remainder)]:
        value = field(call('decode',numeral),'value')
        reading = subprocess.run([str(ROOT/'run_cmds.sh'),'tfactor read '+value],
                                 cwd=ROOT,text=True,capture_output=True,check=True)
        logs.append(label+'\n'+reading.stdout)
        clean = re.sub(r'\x1b\[[0-9;]*m','',reading.stdout)
        hex_word = next(line.split(':',1)[1].strip() for line in clean.splitlines()
                        if 'hex-digit word' in line)
        report.extend([label+' value: `'+value+'`.',
                       label+' word: '+numeral, label+' hex word: '+hex_word])
    print(f'retained={len(keys)} correction={minimum} full_source_equation=PASS exact_placement_return={exact}', flush=True)
record_name = 'transport_plateau_return'+('' if options.complete_return == 0 else '_complete_1')
(HERE/(record_name+'.md')).write_text('\n'.join(report)+'\n')
with gzip.open(HERE/(record_name+'.log.gz'), 'wt') as out:
    out.write('\n'.join(logs))
