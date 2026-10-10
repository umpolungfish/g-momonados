"""Compose certified cross-frame returns with retained signed-copy operations."""
from pathlib import Path
import argparse
import ast
import gzip
import hashlib
import json
import re
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('parent', type=Path)
parser.add_argument('--operand-rewrites', action='store_true',
                    help='Also transport each observed operand-unit zero rewrite before collection.')
args = parser.parse_args()
parent = args.parent.resolve()
assert parent.parent == HERE
logs = []
preserve_odd = False

for filename, names in (
    ('residual_copy_completion.py', {'call', 'field', 'word', 'op', 'normalize'}),
    ('two_sided_partial_return.py', {'cancel', 'magnitude_word', 'certify'}),
    ('combined_zero_partial_return.py', {'fast_step', 'magnitude', 'expanded', 'descend'}),
):
    path = HERE / filename
    nodes = [node for node in ast.parse(path.read_text()).body
             if isinstance(node, ast.FunctionDef) and node.name in names]
    assert {node.name for node in nodes} == names
    exec(compile(ast.Module(body=nodes, type_ignores=[]), str(path), 'exec'), globals())
step = fast_step


def key(*terms):
    return tuple(tuple(sorted(t.items())) for t in terms)


def rewrites(a, b, residual):
    yield a, b, residual, {'operation': 'direct-collection'}
    for side, terms in [('correction', residual)] + (
            [('left', a), ('right', b)] if args.operand_rewrites else []):
        for position, coefficient in sorted(terms.items()):
            if abs(coefficient) != 1:
                continue
            delta = {position: -2*coefficient, position+1: coefficient}
            altered = dict(terms)
            for i, c in delta.items():
                altered[i] = altered.get(i, 0)+c
            altered = {i: c for i, c in altered.items() if c}
            aa, bb, rr = a, b, residual
            if side == 'correction':
                rr = altered
            else:
                other = b if side == 'left' else a
                rr = dict(residual)
                for i, c in delta.items():
                    for j, d in other.items():
                        rr[i+j] = rr.get(i+j, 0)-c*d
                rr = {i: c for i, c in rr.items() if c}
                aa, bb = (altered, b) if side == 'left' else (a, altered)
            yield aa, bb, rr, dict(operation='local-zero-transport', side=side,
                                   position=position, coefficient=coefficient)


words, seeds = {}, {}
summary = None
with gzip.open(parent, 'rt') as stream:
    for line in stream:
        row = json.loads(line)
        if row['kind'] == 'word':
            words[row['id']] = row['support']
        elif row['kind'] == 'target':
            native = row['source_word']
        elif row['kind'] == 'transported_source_equation':
            a = {i: 1 for i in words[row['quotient']]}
            b = {i: 1 for i in words[row['placement']]}
            r = {i: row['correction_sign'] for i in words[row['correction']]}
            seeds.setdefault(key(a, b, r), (a, b, r, []))[3].append(row['path'])
        elif row['kind'] == 'summary':
            summary = row
assert summary is not None and seeds
assert native == field(call('encode', (HERE/'source.txt').read_text().strip()), 'word')

# The native multiplicity control must close before processing the target.
ca, cb, cr = {0: 1, 1: 1}, {2: 1}, {0: 1, 1: 2, 2: 1}
expected, _ = normalize(expanded(ca, cb, cr))
ca, cb, cr, _ = descend(ca, cb, cr, False, False)
assert certify(field(call('encode', '21'), 'word'), ca, cb, cr)[2]

revision = 1
while (HERE/f'transported_signed_collection_{revision}.records.jsonl.gz').exists():
    revision += 1
stem = f'transported_signed_collection_{revision}'
checked, closed, certificates, shapes = 0, 0, set(), set()
with gzip.open(HERE/(stem+'.records.jsonl.gz'), 'wt') as out:
    def emit(kind, **data):
        out.write(json.dumps(dict(kind=kind, **data), separators=(',', ':'))+'\n')

    emit('contract', parent=parent.name, parent_sha256=hashlib.sha256(parent.read_bytes()).hexdigest(),
         operand_rewrites=args.operand_rewrites, source_word=native,
         operation='Canonical and signed-normalized seeds, one observed local zero rewrite, then complete two-sided descent in both orders.')
    for seed_index, (a, b, r, parents) in enumerate(seeds.values()):
        certify(native, a, b, r)
        expected, _ = normalize(expanded(a, b, r))
        emit('seed', index=seed_index, left=sorted(a.items()), right=sorted(b.items()),
             correction=sorted(r.items()), parents=parents)
        variants = {key(a,b,r):(a,b,r)}
        na, nb, nr = (normalize(t)[0] for t in (a,b,r))
        variants[key(na,nb,nr)] = na,nb,nr
        for variant, initial in enumerate(variants.values()):
            for aa, bb, rr, origin in rewrites(*initial):
                for reverse in (False, True):
                    for higher in (False, True):
                        left,right,residual,trace = descend(dict(aa),dict(bb),dict(rr),reverse,higher)
                        assert normalize(expanded(left,right,residual))[0] == expected
                        checked += 1
                        shape = key(left,right,residual)
                        fresh = shape not in shapes
                        shapes.add(shape)
                        normalized = key(*(normalize(t)[0] for t in (left,right,residual)))
                        if normalized not in certificates:
                            wa,wb,is_closed = certify(native,left,right,residual)
                            certificates.add(normalized)
                            if is_closed:
                                result = call('product',native,wa,wb)
                                for label in ('relation.exact-product','relation.codec-assertions','relation.support-carry'):
                                    assert field(result,label) == 'PASS'
                                closed += 1
                                emit('proper_source_product', left=wa, right=wb)
                        emit('path', seed=seed_index, variant=variant, origin=origin,
                             reverse=reverse, higher_first=higher, trace=trace,
                             left=sorted(left.items()), right=sorted(right.items()),
                             correction=sorted(residual.items()), new_shape=fresh,
                             placement_changed=normalize(right)[0] != normalize(b)[0])
                        if checked % 256 == 0:
                            out.flush()
                            print(json.dumps(dict(seed=seed_index, paths=checked,
                                                  certified_returns=len(certificates))), flush=True)
        out.flush()
        print(json.dumps(dict(seed=seed_index,paths=checked,shapes=len(shapes),
                              certified_returns=len(certificates),proper_products=closed)),flush=True)
    emit('summary', seeds=len(seeds), paths=checked, shapes=len(shapes),
         certified_returns=len(certificates), proper_products=closed)
with gzip.open(HERE/(stem+'.native.log.gz'), 'wt') as out:
    out.write('\n'.join(logs))
print(stem+' complete',flush=True)
