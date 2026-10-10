"""Replay scanner and signed transport controls using the production functions."""
from pathlib import Path
import ast
import gzip
import io
import json
import re
import subprocess
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
ONE = (0,)
records = io.StringIO()
native_log = io.StringIO()
started = time.monotonic()
word_ids, catalog, source_returns = {}, {}, {}
scanned_components = set()
stats = dict(component_products=0, component_words=0, component_fold_tests=0,
             fold_joins=0, overlay_joins=0, cross_width_comparisons=0,
             cross_width_exact_returns=0)

for filename, selected in (
    ('cross_frame_collection.py', None),
    ('placement_full_return.py', {'frame'}),
    ('cross_frame_product_transport.py', {'enrich'}),
):
    path = HERE / filename
    nodes = [node for node in ast.parse(path.read_text()).body
             if isinstance(node, ast.FunctionDef)
             and (selected is None or node.name in selected)]
    exec(compile(ast.Module(body=nodes, type_ignores=[]), str(path), 'exec'), globals())

source = native_support('45')
three, fifteen, five, nine = map(native_support, ('3', '15', '5', '9'))
register(source, three, fifteen, {'operation': 'preloaded-control'})
structural_returns(source)
assert catalog[source][three][0] == fifteen
assert catalog[source][five][0] == nine
before = stats['component_words']
structural_returns(source)
assert stats['component_words'] == before
emit('scanner_control', existing_return_preserved=True,
     alternative_return_found=True, completed_scan_not_repeated=True)

catalog.clear()
scanned_components.clear()
source_returns.clear()
source = native_support('175')
rows = []
for w, a, b in ((2, '15', '12'), (3, '9', '20')):
    k, q, r = map(native_support, (a, b, '5'))
    equation_certificate(source, k, q, -1, r)
    rows.append((w, 1, 'negative-correction-control', k, q, -1, r))
    for component in (k, q, r):
        structural_returns(component)
enrich(source, rows)
events = [json.loads(line) for line in records.getvalue().splitlines()]
transported = [row for row in events if row['kind'] == 'transported_source_equation']
assert any(row['correction_sign'] == -1 for row in transported)
assert any(row['placement'] == wid(native_support('135'))
           and row['quotient'] == wid(ONE)
           and row['correction_sign'] == 1
           and row['correction'] == wid(native_support('40')) for row in transported)
emit('signed_transport_control', negative_correction_preserved=True,
     nonzero_remainder_changes_correction_sign=True,
     complete_source_equations_verified=True)
for suffix, contents in (('records.jsonl.gz', records.getvalue()),
                         ('native.log.gz', native_log.getvalue())):
    with gzip.open(HERE / ('cross_frame_transport_controls.' + suffix), 'wt') as out:
        out.write(contents)
print('Scanner alternative-return and signed parent-transport controls PASS')
