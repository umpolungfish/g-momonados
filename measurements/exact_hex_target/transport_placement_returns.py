"""Check complete returns of the observed transport-branch placements."""
from pathlib import Path
import ast
import gzip
import re
import subprocess

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
load_functions(HERE/'two_sided_partial_return.py', ['magnitude_word'])
load_functions(HERE/'placement_full_return.py', ['frame'])
source = field(call('encode', (HERE/'source.txt').read_text().strip()), 'word')
text = (HERE/'transport_branch_return.md').read_text()
branches = re.findall(r'Endpoint operand B: (\[.*\])', text)
assert len(branches) == 8
placements = {}
for index, branch in enumerate(branches):
    terms = dict(ast.literal_eval(branch))
    sign, placement = magnitude_word(terms)
    placements.setdefault(placement, []).append(index)
report = ['# Complete observed transport-placement returns', '',
          'Run: `python3 measurements/exact_hex_target/transport_placement_returns.py`.', '',
          'Each placement comes from a retained operand-transport branch. '
          'Native Gödel division supplies its complete source quotient and remainder. '
          'Separate multiplication and addition checks reconstruct the source.', '']
for placement, indexes in placements.items():
    division = frame(source, 'divmod', placement)
    quotient = field(call('encode', field(division, 'result')), 'word')
    remainder = field(call('encode', field(division, 'remainder')), 'word')
    product = op(placement, 'mul', quotient)
    assert op(product, 'add', remainder) == source
    exact = remainder == word([])
    report.extend(['Branch indexes: '+repr(indexes)+'.',
                   'Exact placement return: '+str(exact)+'.'])
    for label, numeral in [('placement', placement), ('quotient', quotient), ('remainder', remainder)]:
        report.extend([label+' value: `'+field(call('decode', numeral), 'value')+'`.',
                       label+' word: '+numeral])
    report.append('')
    print(f'branches={indexes} full_source_equation=PASS exact_placement_return={exact}', flush=True)
(HERE/'transport_placement_returns.md').write_text('\n'.join(report)+'\n')
with gzip.open(HERE/'transport_placement_returns.log.gz', 'wt') as out:
    out.write('\n'.join(logs))
