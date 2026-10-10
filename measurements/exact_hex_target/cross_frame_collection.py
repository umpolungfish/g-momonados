"""Compose retained frame folds, component returns and positioned frame overlays.

The host manipulates cell positions and signed contribution coefficients. Native
Gödel checks every root fold, every registered component product and every emitted
proper source product. No unknown operand cells or numerical factor solver enter.
"""
from pathlib import Path
import argparse
import gzip
import hashlib
import json
import re
import subprocess
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
ONE = (0,)
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--controls-only', action='store_true')
args = parser.parse_args()
stem = 'cross_frame_collection_controls' if args.controls_only else 'cross_frame_collection'
revision = 1
while (HERE / f'{stem}_{revision}.records.jsonl.gz').exists():
    revision += 1
name = f'{stem}_{revision}'
native_log = gzip.open(HERE / (name + '.native.log.gz'), 'wt')
records = gzip.open(HERE / (name + '.records.jsonl.gz'), 'wt')
started = time.monotonic()
stats = dict(root_folds=0, collected_nonzero_folds=0, component_words=0,
             component_fold_tests=0, component_products=0, fold_joins=0,
             overlay_pairs=0, overlay_fragments=0, overlay_joins=0)
word_ids, catalog = {}, {}
scanned_components = set()
source_returns = {}


def emit(kind, **data):
    records.write(json.dumps(dict(kind=kind, **data), separators=(',', ':')) + '\n')


def status(stage, **data):
    message = dict(stage=stage, elapsed_seconds=round(time.monotonic()-started, 1))
    message.update(data)
    print(json.dumps(message), flush=True)
    records.flush()
    native_log.flush()


def call(*command):
    result = subprocess.run([str(ROOT/'godel'), *command], cwd=ROOT,
                            capture_output=True, text=True)
    native_log.write(repr(command)+'\n'+result.stdout+result.stderr+
                     '\nEXIT '+str(result.returncode)+'\n')
    assert result.returncode == 0, result.stdout + result.stderr
    return result.stdout


def field(text, label):
    return next(line.split(None, 1)[1] for line in text.splitlines()
                if re.match('^'+re.escape(label)+r'\s', line))


def word(support):
    occupied = set(support)
    return '⊢'+''.join(FILLED if i in occupied else EMPTY
                      for i in range(max(occupied, default=0)+1))+'⊙⊡⊣'


def native_support(value):
    body = field(call('encode', value), 'word')[1:-3]
    return tuple(i for i in range(len(body)//5) if body[5*i:5*i+5] == FILLED)


def width(support):
    return support[-1]+1 if support else 0


def dense_signed(coefficients):
    """Canonical magnitude via local binary carries, including negative words."""
    out, carry = [], 0
    for i, coefficient in enumerate(coefficients):
        current = coefficient + carry
        if current % 2:
            out.append(i)
        carry = current // 2
    i = len(coefficients)
    while carry not in (0, -1):
        if carry % 2:
            out.append(i)
        carry //= 2
        i += 1
    if carry == -1:
        sign, result = dense_signed([-c for c in coefficients])
        assert sign >= 0
        return -1, result
    return (1 if out else 0), tuple(out)


def combine(*parts):
    coefficients = [0]*max((width(support) for _, support in parts), default=0)
    for sign, support in parts:
        for i in support:
            coefficients[i] += sign
    return dense_signed(coefficients)


def add(a, b):
    sign, result = combine((1, a), (1, b))
    assert sign >= 0
    return result


def mul(a, b):
    if not a or not b:
        return ()
    coefficients = [0]*(width(a)+width(b))
    for i in a:
        for j in b:
            coefficients[i+j] += 1
    return dense_signed(coefficients)[1]


def wid(support):
    support = tuple(support)
    if support not in word_ids:
        word_ids[support] = len(word_ids)
        emit('word', id=word_ids[support], support=support)
    return word_ids[support]


def product_certificate(source, a, b):
    out = call('product', word(source), word(a), word(b))
    for label in ('relation.exact-product', 'relation.codec-assertions', 'relation.support-carry'):
        assert field(out, label) == 'PASS', out


def equation_certificate(source, k, q, sign, residual):
    product = mul(k, q)
    assert combine((1, product), (sign, residual)) == (1, source)
    out = call('check', 'mul', word(k), word(q), word(product))
    assert 'PASS' in out and 'FAIL' not in out
    a, b, c = ((product, residual, source) if sign >= 0
               else (source, residual, product))
    out = call('check', 'add', word(a), word(b), word(c))
    assert 'PASS' in out and 'FAIL' not in out


def register(source, a, b, path):
    if not a or not b or a == ONE or b == ONE:
        return
    index = catalog.setdefault(source, {})
    if a in index:
        assert index[a][0] == b
        return
    product_certificate(source, a, b)
    index[a] = (b, path)
    index[b] = (a, path)
    stats['component_products'] += 1
    emit('component_product', source=wid(source), left=wid(a), right=wid(b), path=path)


def known_factors(source):
    if not source:
        return {}
    found = dict(catalog.get(source, {}))
    if source != ONE:
        found[source] = (ONE, dict(operation='whole-component'))
    return found


def return_source(source, a, b, path):
    if not a or not b or a == ONE or b == ONE or a == source or b == source:
        return False
    key = tuple(sorted((a, b)))
    if key in source_returns:
        return True
    product_certificate(source, a, b)
    source_returns[key] = path
    emit('proper_source_product', source=wid(source), left=wid(a), right=wid(b), path=path)
    status('proper_source_product', left=field(call('decode', word(a)), 'value'),
           right=field(call('decode', word(b)), 'value'))
    return True


def folded(source, w, epsilon, quotient=True):
    """N=(2^w-epsilon)*Q+sum epsilon^j*frame_j."""
    residue = [0]*w
    q = [0]*max(0, width(source)-w) if quotient else None
    for i in source:
        group, position = divmod(i, w)
        residue[position] += 1 if epsilon == 1 or group % 2 == 0 else -1
        if quotient:
            for power in range(group):
                q[position+power*w] += (1 if epsilon == 1 or (group-1-power) % 2 == 0 else -1)
    rsign, r = dense_signed(residue)
    if not quotient:
        return rsign, r
    qsign, qs = dense_signed(q)
    assert qsign >= 0
    return qs, rsign, r


def kernel(w, epsilon):
    return tuple(range(w)) if epsilon == 1 else (0, w)


def complete_fold(source, w, epsilon):
    k = kernel(w, epsilon)
    q, rsign, r = folded(source, w, epsilon)
    original = (q, rsign, r)
    trace = []
    while r and (width(r) > w or r == k):
        before = r
        if r == k:
            extra, next_sign, next_r = ONE, 0, ()
        else:
            extra, next_sign, next_r = folded(r, w, epsilon)
        qsign, q = combine((1, q), (rsign, extra))
        assert qsign >= 0
        trace.append(dict(input=wid(r), sign=rsign, quotient=wid(extra),
                          next_sign=rsign*next_sign, remainder=wid(next_r)))
        rsign, r = rsign*next_sign, next_r
        assert not r or tuple(reversed(r)) < tuple(reversed(before))
    if rsign < 0:
        qsign, q = combine((1, q), (-1, ONE))
        assert qsign >= 0
        positive, r = combine((1, k), (-1, r))
        assert positive > 0
        rsign = 1
        trace.append(dict(operation='negative-remainder-complement', remainder=wid(r)))
    return original, (q, rsign, r), trace


def fold_closes(source, w, epsilon):
    """Read only the correction until an exact component return is established."""
    sign, residual = folded(source, w, epsilon, quotient=False)
    k = kernel(w, epsilon)
    while residual and width(residual) > w and residual != k:
        previous = residual
        next_sign, residual = folded(residual, w, epsilon, quotient=False)
        sign *= next_sign
        assert not residual or tuple(reversed(residual)) < tuple(reversed(previous))
    return not residual or residual == k


def structural_returns(source):
    """Every width, including nonzero collected fold imbalances, on one component."""
    if not source or source == ONE or source in scanned_components:
        return
    catalog.setdefault(source, {})
    stats['component_words'] += 1
    size, occupied = width(source), set(source)
    body = bytes(1 if i in occupied else 0 for i in range(size))
    if source[0]:
        register(source, (source[0],), tuple(i-source[0] for i in source),
                 dict(operation='initial-empty-cells', shift=source[0]))
    for w in range(1, size):
        payload, starts = None, []
        for start in range(0, size, w):
            frame = body[start:start+w].ljust(w, b'\0')
            if 1 not in frame:
                continue
            if payload is None:
                payload = frame
            elif payload != frame:
                break
            starts.append(start)
        else:
            a = tuple(i for i, bit in enumerate(payload or b'') if bit)
            register(source, a, tuple(starts), dict(operation='shared-occupied-frames', width=w))
        for epsilon in (1, -1):
            if w == 1 and epsilon == 1:
                continue
            stats['component_fold_tests'] += 1
            if fold_closes(source, w, epsilon):
                original, terminal, trace = complete_fold(source, w, epsilon)
                q, _, r = terminal
                assert not r
                register(source, kernel(w, epsilon), q,
                         dict(operation='collected-frame-fold', width=w, epsilon=epsilon,
                              original_quotient=wid(original[0]), correction_sign=original[1],
                              original_correction=wid(original[2]), trace=trace))
    scanned_components.add(source)
    emit('component_scan', source=wid(source), widths=[1, max(0, size-1)],
         returned_components=[wid(k) for k in catalog[source]],
         failed_shapes='Reconstruct from this source, width and epsilon with folded(); none discarded as zero.')


def join_fold(source, k, q, rsign, r, path):
    """Match returns from either operand or the whole product with the residual."""
    if not r:
        return_source(source, k, q, dict(**path, collection='zero-correction'))
        return
    residual_factors = known_factors(r)
    for operand, other, side in ((k, q, 'placement'), (q, k, 'quotient')):
        shared = known_factors(operand).keys() & residual_factors.keys()
        for d in shared:
            a = known_factors(operand)[d][0]
            b = residual_factors[d][0]
            sign, c = combine((1, mul(a, other)), (rsign, b))
            assert sign > 0
            stats['fold_joins'] += 1
            return_source(source, d, c, dict(**path, collection=side+'-residual',
                                            component=wid(d)))
    term = mul(k, q)
    for d in known_factors(term).keys() & residual_factors.keys():
        a = known_factors(term)[d][0]
        b = residual_factors[d][0]
        sign, c = combine((1, a), (rsign, b))
        assert sign > 0
        stats['fold_joins'] += 1
        return_source(source, d, c, dict(**path, collection='positioned-product-residual',
                                        component=wid(d)))


def root_folds(source):
    rows, components = [], set()
    for w in range(1, width(source)):
        frames = [tuple(i-start for i in source if start <= i < start+w)
                  for start in range(0, width(source), w)]
        emit('source_frames', width=w, frames=[dict(start=j*w, end=min((j+1)*w, width(source)),
                                                   payload=wid(frame)) for j, frame in enumerate(frames)])
        for epsilon in (1, -1):
            k = kernel(w, epsilon)
            original, terminal, trace = complete_fold(source, w, epsilon)
            for tag, (q, sign, r) in (('original', original), ('collected', terminal)):
                equation_certificate(source, k, q, sign, r)
                rows.append((w, epsilon, tag, k, q, sign, r))
                components.update((k, q, r))
            stats['root_folds'] += 1
            if original[2] and not terminal[2]:
                stats['collected_nonzero_folds'] += 1
            emit('root_fold', width=w, epsilon=epsilon, placement=wid(k),
                 original=dict(quotient=wid(original[0]), sign=original[1], residual=wid(original[2])),
                 collected=dict(quotient=wid(terminal[0]), sign=terminal[1], residual=wid(terminal[2])),
                 trace=trace)
        if w % 64 == 0:
            status('root_folds', width=w, folds=stats['root_folds'])
    return rows, components


def overlays(source):
    """Every pair of proper widths, with original positions and child matches."""
    size = width(source)
    occupied = set(source)
    body = bytes(1 if i in occupied else 0 for i in range(size))
    for w in range(1, size-1):
        for v in range(w+1, size):
            boundaries = sorted(set(range(0, size, w)) | set(range(0, size, v)) | {size})
            groups, fragments = {}, []
            restored = []
            for start, end in zip(boundaries, boundaries[1:]):
                fragment = tuple(i for i, bit in enumerate(body[start:end]) if bit)
                fragments.append((start, end, wid(fragment)))
                if fragment:
                    groups.setdefault(fragment, []).append(start)
                    restored.extend(i+start for i in fragment)
            assert tuple(restored) == source
            stats['overlay_pairs'] += 1
            stats['overlay_fragments'] += len(fragments)
            common, terms = None, []
            for payload, starts in groups.items():
                placement = tuple(starts)
                placed = tuple(sorted(i+start for start in starts for i in payload))
                assert len(placed) == len(set(placed))
                options = {placed:(ONE, dict(operation='whole-positioned-term'))}
                for operand, other, side in ((payload, placement, 'payload'),
                                              (placement, payload, 'placement')):
                    for d, (cofactor, child_path) in known_factors(operand).items():
                        options[d] = (None, (cofactor, other, side, child_path))
                for d, value in known_factors(placed).items():
                    options[d] = value
                options.pop(ONE, None)
                terms.append((payload, placement, placed, options))
                common = set(options) if common is None else common & options.keys()
            for d in common or ():
                cofactor = ()
                for _, _, _, options in terms:
                    c, path = options[d]
                    if c is None:
                        c = mul(path[0], path[1])
                    cofactor = add(cofactor, c)
                stats['overlay_joins'] += 1
                return_source(source, d, cofactor, dict(operation='overlay-common-component',
                                                        widths=[w, v], component=wid(d)))
            emit('overlay', widths=[w, v], fragments=fragments,
                 groups=[dict(payload=wid(a), starts=b) for a, b, _, _ in terms],
                 common_components=[wid(d) for d in common or ()], exact_position_return=True)
        if w % 32 == 0:
            status('overlays', first_width=w, pairs=stats['overlay_pairs'])


def controls():
    source = native_support('45')
    three, fifteen = native_support('3'), native_support('15')
    five, nine = native_support('5'), native_support('9')
    register(source, three, fifteen, dict(operation='preloaded-control-return'))
    structural_returns(source)
    assert catalog[source][three][0] == fifteen
    assert catalog[source][five][0] == nine
    scans = stats['component_words']
    structural_returns(source)
    assert stats['component_words'] == scans
    # Nonzero fold collection, common-component collection and crossed framings.
    for value in ('51', '117', '213'):
        source = native_support(value)
        rows, components = root_folds(source)
        for component in sorted(components, key=lambda x:(width(x), x)):
            structural_returns(component)
        before = len(source_returns)
        for w, epsilon, tag, k, q, sign, r in rows:
            join_fold(source, k, q, sign, r,
                      dict(operation='control-fold', source=value, width=w, epsilon=epsilon, form=tag))
        overlays(source)
        assert len(source_returns) > before
    source = native_support('117')
    original, terminal, _ = complete_fold(source, 2, 1)
    assert original[2] == native_support('6') and not terminal[2]
    # Native bad-source and unit controls exercise different certificate fields.
    a, b = native_support('3'), native_support('17')
    bad = call('product', '52', word(a), word(b))
    assert field(bad, 'relation.exact-product') == 'FAIL'
    unit = call('product', '51', '1', '51')
    assert field(unit, 'relation.exact-product') == 'PASS'
    assert not return_source(native_support('51'), ONE, native_support('51'), {})
    emit('controls_complete', nonzero_correction_collection=True,
         source_component_joins=True, overlay_position_returns=True,
         wrong_source_rejected=True, unit_rejected=True)
    status('controls_pass')


try:
    emit('contract', script_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
         root_fold_widths='1 through source cell count minus one, zero source offset',
         component_operations=['initial-empty-cells', 'shared occupied frames',
                               'both frame folds including collected nonzero imbalances'],
         component_depth='one complete structural pass over all root fold operands and corrections',
         overlay_operations='every pair of proper widths; literal grouping and matching registered component returns',
         scope='No arbitrary offset sweep, unknown factor support search, or claim of saturation of all notebook operations.')
    controls()
    if not args.controls_only:
        source_returns.clear()
        catalog.clear()
        scanned_components.clear()
        stats.update({key:0 for key in stats})
        source = native_support((HERE/'source.txt').read_text().strip())
        emit('target', source=wid(source), source_word=word(source))
        rows, components = root_folds(source)
        status('root_folds_complete', components=len(components), **stats)
        for index, component in enumerate(sorted(components, key=lambda x:(width(x), x))):
            structural_returns(component)
            if index % 32 == 0:
                status('component_returns', processed=index+1, total=len(components),
                       products=stats['component_products'], fold_tests=stats['component_fold_tests'])
        for w, epsilon, tag, k, q, sign, r in rows:
            join_fold(source, k, q, sign, r,
                      dict(operation='cross-frame-component-collection', width=w, epsilon=epsilon, form=tag))
        status('fold_joins_complete', **stats, proper_source_products=len(source_returns))
        overlays(source)
    summary = dict(stats=stats, proper_source_products=len(source_returns),
                   elapsed_seconds=round(time.monotonic()-started, 3),
                   factor_extraction_evidence='True' if source_returns else 'None')
    emit('summary', **summary)
    (HERE/(name+'.md')).write_text(
        '# Retained folds and cross-frame collection\n\n'
        + 'Command: `python3 measurements/exact_hex_target/cross_frame_collection.py'
        + (' --controls-only' if args.controls_only else '')+'`.\n\n'
        + 'Complete positioned words, original and collected folds, component paths, '
        + 'and every overlay are in `'+name+'.records.jsonl.gz`. '
        + 'Native checks are in `'+name+'.native.log.gz`.\n\n'
        + '```json\n'+json.dumps(summary,indent=2)+'\n```\n')
    status('complete', **summary)
except BaseException as error:
    emit('interrupted', error=repr(error), stats=stats)
    raise
finally:
    records.close()
    native_log.close()
