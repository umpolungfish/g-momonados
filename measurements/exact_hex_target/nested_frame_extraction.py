"""Compose source-frame extraction and three exact lifts, without factor guesses."""
from pathlib import Path
from functools import lru_cache
import argparse
import ast
import gzip
import json
import subprocess
import re

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--depth', type=int, default=2, choices=(1, 2, 3))
parser.add_argument('--saturate', action='store_true',
                    help='Compose lifts through every strictly shorter child word, without a nesting-depth cutoff.')
parser.add_argument('--base', choices=('shared','full'), default='full',
                    help='Include filled-frame sum and alternating-balance child returns in the full base.')
options = parser.parse_args()
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
logs = []
stats = dict(nodes=0, lifts=0, structural_products=0, lifted_products=0)
child_products = {}


def load_functions(path, names):
    tree = ast.parse(path.read_text())
    nodes = [n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name in names]
    exec(compile(ast.Module(body=nodes, type_ignores=[]), str(path), 'exec'), globals())


load_functions(HERE/'residual_copy_completion.py', ['call', 'field', 'word', 'op', 'normalize'])
ONE = (0,)


def positive_support(terms):
    coefficients = dict(terms)
    result = []
    ceiling = max(coefficients,default=0)+sum(abs(c) for c in coefficients.values()).bit_length()+3
    for i in range(ceiling+1):
        c = coefficients.get(i,0)
        if c % 2:
            result.append(i)
        if c//2:
            coefficients[i+1] = coefficients.get(i+1,0)+c//2
    assert not coefficients.get(ceiling+1,0)
    return tuple(result)


def signed(terms):
    if all(c >= 0 for c in terms.values()):
        result = positive_support(terms)
        return (1 if result else 0), result
    reduced, _ = normalize(terms)
    if not reduced:
        return 0, ()
    sign = 1 if reduced[max(reduced)] > 0 else -1
    coefficients = {i:sign*c for i,c in reduced.items()}
    result = []
    ceiling = max(coefficients)+3
    for i in range(ceiling+1):
        c = coefficients.get(i, 0)
        if c % 2:
            result.append(i)
        carry = c//2
        if carry:
            coefficients[i+1] = coefficients.get(i+1, 0)+carry
    assert not coefficients.get(ceiling+1, 0)
    return sign, tuple(result)


def combine(*parts):
    # Canonical support order determines the sign of a two-word difference.
    # Carry/borrow then runs once, with no numerical evaluation of the word.
    if len(parts) == 2 and parts[0][0] == 1 and parts[1][0] in (1,-1):
        a,b = parts[0][1],parts[1][1]
        coefficient = parts[1][0]
        sign = 1
        if coefficient == -1:
            if a == b:
                return 0, ()
            if tuple(reversed(a)) < tuple(reversed(b)):
                a,b,sign = b,a,-1
        terms = {i:1 for i in a}
        for i in b:
            terms[i] = terms.get(i,0)+coefficient
        result = positive_support(terms)
        return (sign if result else 0), result
    terms = {}
    for coefficient, support in parts:
        for i in support:
            terms[i] = terms.get(i, 0)+coefficient
    return signed(terms)


def add(a, b):
    sign, result = combine((1,a), (1,b))
    assert sign >= 0
    return result


def shift(a, r):
    return tuple(i+r for i in a)


def bits(a):
    return a[-1]+1 if a else 0


def product_support(a, b):
    terms = {}
    for i in a:
        for j in b:
            terms[i+j] = terms.get(i+j, 0)+1
    sign, result = signed(terms)
    assert sign >= 0
    return result


def matches_filled_sum(coefficients, width):
    carry = 0
    for i in range(width):
        c = coefficients.get(i,0)+carry
        if c % 2 != 1:
            return False
        carry = c//2
    return carry == 0


def matches_zero(coefficients, width):
    carry = 0
    for i in range(width):
        c = coefficients.get(i,0)+carry
        if c % 2:
            return False
        carry = c//2
    return carry == 0


def candidate_shifts(width, high, third, subtract):
    # In X+Y=Z, the two longest positive words differ by at most one cell.
    # Their lengths here are upper_bits+r, width-r+1 and third_bits.
    upper_bits,third_bits = bits(high),bits(third)
    candidates = {third[0]-high[0]} if third else set()
    if subtract and third:
        candidates.add(width-third[0])
    if subtract or not third:
        if (width-high[0]) % 2 == 0:
            candidates.add((width-high[0])//2)
    for r in sorted(candidates):
        if not 0 <= r < width:
            continue
        lengths = sorted((upper_bits+r, width-r+1, third_bits))
        if lengths[-1]-lengths[-2] <= 1:
            yield r


@lru_cache(None)
def base_returns(source):
    found = {}
    if not source:
        return ()
    width = bits(source)
    if source[0] > 0:
        pair = (shift(ONE,source[0]), tuple(i-source[0] for i in source))
        found[tuple(sorted(pair))] = dict(operation='initial-empty-cells', shift=source[0])
    occupied = set(source)
    body = tuple(i in occupied for i in range(width))
    for w in range(1, width):
        payload, placements = None, []
        for start in range(0, width, w):
            frame = body[start:start+w]
            frame += (False,)*(w-len(frame))
            if not any(frame):
                continue
            if payload is None:
                payload = frame
            elif frame != payload:
                break
            placements.append(start)
        else:
            a = tuple(i for i,occupied in enumerate(payload or ()) if occupied)
            b = tuple(placements)
            if a and b and a != ONE and b != ONE:
                found[tuple(sorted((a,b)))] = dict(operation='shared-occupied-frames', width=w)
        if options.base == 'shared':
            continue
        totals, alternating = {}, {}
        for i in source:
            position = i % w
            totals[position] = totals.get(position,0)+1
            alternating[position] = alternating.get(position,0)+(1 if (i//w)%2 == 0 else -1)
        if w > 1 and matches_filled_sum(totals,w):
            fold = {0:1}
            for i in source:
                for j in range(i-w,-1,-w):
                    fold[j] = fold.get(j,0)+1
            a,b = tuple(range(w)),positive_support(fold)
            found[tuple(sorted((a,b)))] = dict(operation='filled-frame-sum',width=w)
        if matches_zero(alternating,w):
            # T_j = D^(j-1)-D^(j-2)+...+(-1)^(j-1), so
            # source = (D+1)*sum(frame_j*T_j) when alternating sum is zero.
            fold = {}
            for i in source:
                group,position = divmod(i,w)
                for power in range(group):
                    j = position+power*w
                    fold[j] = fold.get(j,0)+(1 if (group-1-power)%2 == 0 else -1)
            sign,b = signed(fold)
            assert sign == 1
            a = (0,w)
            found[tuple(sorted((a,b)))] = dict(operation='alternating-frame-balance',width=w)
    result = []
    for (a,b),path in found.items():
        if a == ONE or b == ONE:
            continue
        assert product_support(a,b) == source
        result.append((a,b,path))
        child_products[(source,a,b)] = path
        stats['structural_products'] += 1
    return tuple(result)


@lru_cache(None)
def extract(source, depth):
    stats['nodes'] += 1
    found = {(a,b):(a,b,path) for a,b,path in base_returns(source)}
    if depth == 0:
        return tuple(found.values())
    width = bits(source)
    for w in range(1, width):
        lower = tuple(i for i in source if i < w)
        upper = tuple(i-w for i in source if i >= w)
        if not lower or not upper:
            continue
        # A positive lower pair has a+b <= L+1; a gap pair has
        # |a-b| <= M-1. Their words have at most w+1 cells. An upper
        # word more than one cell longer than both cannot close any lift.
        if bits(upper) > w+2:
            continue
        _, gap = combine((1,(w,)), (-1,lower))
        for mode, child, high in (('add',lower,upper), ('subtract',lower,upper),
                                  ('mixed',gap,add(upper,ONE))):
            pairs = [(ONE,child,dict(operation='unit-lower-return'))]
            pairs.extend(extract(child,bits(child) if options.saturate else depth-1))
            for a,b,path in pairs:
                orientations = ((a,b),(b,a)) if mode == 'mixed' else ((a,b),)
                for a,b in orientations:
                    sum_sign, third = combine((1,a), (-1 if mode == 'mixed' else 1,b))
                    subtract = mode == 'subtract' or (mode == 'mixed' and sum_sign < 0)
                    for r in candidate_shifts(w,high,third,subtract):
                        stats['lifts'] += 1
                        c, boundary = shift(high,r), (w-r,)
                        if mode == 'add':
                            if add(c,third) != boundary:
                                continue
                            p,q = add(c,a),add(c,b)
                        elif mode == 'subtract':
                            if combine((1,c),(-1,third)) != (1,boundary):
                                continue
                            sp,p = combine((1,c),(-1,a))
                            sq,q = combine((1,c),(-1,b))
                            if sp != 1 or sq != 1:
                                continue
                        else:
                            if combine((1,c),(sum_sign,third)) != (1,boundary):
                                continue
                            p = add(c,a)
                            sq,q = combine((1,c),(-1,b))
                            if sq != 1:
                                continue
                        if p == ONE or q == ONE:
                            continue
                        assert product_support(p,q) == source
                        p,q = sorted((p,q))
                        found[(p,q)] = (p,q,dict(operation=mode+'-frame-lift',
                            width=w, upper_shift=r, child=path,
                            lower_pair=[a,b], upper=upper))
                        child_products[(source,p,q)] = found[(p,q)][2]
                        stats['lifted_products'] += 1
        if stats['nodes'] % 100 == 0 and w == width-1:
            print('progress '+json.dumps(stats),flush=True)
    return tuple(found.values())


def native_certificate(source, a, b):
    native = word(source)
    product = op(word(a), 'mul', word(b))
    assert product == native
    assert 'PASS' in call('check','mul',word(a),word(b),native)


def native_support(value):
    native = field(call('encode',value),'word')
    body = native[1:-3]
    return tuple(i for i in range(len(body)//5) if body[5*i:5*i+5] == FILLED)


# Controls exercise exact positive, subtractive and mixed source-frame lifts.
controls = []
for value in ('21','91','99','143','221'):
    source = native_support(value)
    returns = extract(source,1)
    assert returns
    for a,b,path in returns:
        native_certificate(source,a,b)
    controls.append(dict(source=value,returns=returns))
print('all native lift controls PASS',flush=True)
source = native_support((HERE/'source.txt').read_text().strip())
stats.update({key:0 for key in stats})
child_products.clear()
extract.cache_clear()
base_returns.cache_clear()
returns = extract(source,bits(source) if options.saturate else options.depth)
for a,b,path in returns:
    native_certificate(source,a,b)
for (child,a,b),path in child_products.items():
    native_certificate(child,a,b)
name = 'nested_frame_extraction_saturated' if options.saturate else 'nested_frame_extraction_depth_'+str(options.depth)
if options.base == 'full':
    name += '_full'
report = dict(source_word=word(source), depth=None if options.saturate else options.depth,
              saturated=options.saturate, base_operations=options.base, controls=controls,
              counts=stats, proper_returns=returns,
              certified_child_products=[dict(source=child, left=a, right=b, path=path)
                                        for (child,a,b),path in child_products.items()],
              factor_extraction_evidence='True' if returns else 'None',
              requested_product=dict(source_word=word(source),
                                     left_operand='unresolved proper operand word' if not returns else None,
                                     right_operand='unresolved proper operand word' if not returns else None),
              selection='Only complete source-frame products and exact lift identities; no unknown factor cells, divisibility candidates or conventional factor solver.')
(HERE/(name+'.json')).write_text(json.dumps(report,indent=2)+'\n')
with gzip.open(HERE/(name+'.log.gz'),'wt') as archive:
    archive.write('\n'.join(logs))
print(json.dumps(dict(counts=stats,proper_source_products=len(returns))),flush=True)
