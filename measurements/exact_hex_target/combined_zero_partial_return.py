"""Combine intermediate zero rewrites with two-sided collection and cancellation."""
from pathlib import Path
import runpy
import gzip
import subprocess
import re
import argparse

parser = argparse.ArgumentParser()
parser.add_argument('--fast', action='store_true')
options = parser.parse_args()

HERE = Path(__file__).resolve().parent
saved = {p:p.read_bytes() for p in [HERE/'two_sided_partial_return.md',
                                  HERE/'two_sided_partial_return.log.gz']}
try:
    state = runpy.run_path(str(HERE/'two_sided_partial_return.py'))
finally:
    for path, contents in saved.items():
        path.write_bytes(contents)
step, cancel, certify, normalize = (state[k] for k in ('step','cancel','certify','normalize'))
reference_step = step


def fast_step(a, b, residual, reverse):
    before = sum(abs(c) for c in residual.values())
    best = None
    for side,payload in enumerate((a,b)):
        if not payload:
            continue
        baseline = sum(abs(c) for c in payload.values())
        scores = {}
        for i,c in payload.items():
            for j,r in residual.items():
                if j < i:
                    continue
                shift = j-i
                pair = scores.setdefault(shift, [baseline, baseline])
                pair[0] += abs(r-c)-abs(r)-abs(c)
                pair[1] += abs(r+c)-abs(r)-abs(c)
        for shift in sorted(scores, reverse=reverse):
            for sign,delta in zip((1,-1),scores[shift]):
                if delta < 0 and (best is None or delta < best[0]):
                    best = delta,side,shift,sign
    if best is None:
        return None
    delta,side,shift,sign = best
    payload = a if side == 0 else b
    changed = dict(residual)
    for i,c in payload.items():
        changed[i+shift] = changed.get(i+shift,0)-sign*c
        if not changed[i+shift]:
            del changed[i+shift]
    updated = dict(b if side == 0 else a)
    updated[shift] = updated.get(shift,0)+sign
    if not updated[shift]:
        del updated[shift]
    after = sum(abs(c) for c in changed.values())
    assert after == before+delta and after < before
    left,right = (a,updated) if side == 0 else (updated,b)
    return left,right,changed,dict(payloadSide=side,shift=shift,sign=sign,
        beforeMagnitude=before,afterMagnitude=after,correction=sorted(changed.items()))
logs = state['logs']
A, B, R = (dict(state[k]) for k in ('A0','B0','R0'))
if options.fast:
    # Compare the accelerated overlap score with the recorded selector on
    # every initial one-unit rewrite in both shift orders before using it.
    for position,c in sorted(R.items()):
        altered = dict(R)
        altered[position] = -c
        altered[position+1] = altered.get(position+1,0)+c
        altered = {i:d for i,d in altered.items() if d}
        for reverse in (False,True):
            assert fast_step(A,B,altered,reverse) == reference_step(A,B,altered,reverse)
    step = fast_step
    print('accelerated selector matches every initial observed-unit control', flush=True)
native = state['native']
ROOT = state['ROOT']


def magnitude(terms):
    return sum(abs(c) for c in terms.values())


def expanded(a, b, residual):
    result = dict(residual)
    for i,c in a.items():
        for j,d in b.items():
            result[i+j] = result.get(i+j, 0)+c*d
    return {i:c for i,c in result.items() if c}


expected, _ = normalize(expanded(A, B, R))


def descend(a, b, residual, reverse, higher):
    trace = []
    while True:
        before = magnitude(residual)
        result = step(a, b, residual, reverse)
        move = None
        if result is not None:
            a, b, residual, move = result
        residual, rewrites = cancel(residual, higher)
        if result is None and not rewrites:
            break
        assert magnitude(residual) < before
        trace.append((move, rewrites, sorted(a.items()), sorted(b.items()), sorted(residual.items())))
    actual, _ = normalize(expanded(a, b, residual))
    assert actual == expected
    return a, b, residual, trace


rounds, comparisons = [], []
checked = 0
while R:
    score = magnitude(R)
    best = None
    for position,c in sorted(R.items()):
        if abs(c) != 1:
            continue
        altered = dict(R)
        altered[position] = -c
        altered[position+1] = altered.get(position+1, 0)+c
        altered = {i:d for i,d in altered.items() if d}
        for reverse in (False, True):
            for higher in (False, True):
                a,b,r,trace = descend(dict(A), dict(B), dict(altered), reverse, higher)
                checked += 1
                final = magnitude(r)
                comparisons.append((len(rounds),position,c,reverse,higher,final))
                logs.append('SYMBOLIC PATH '+repr((len(rounds),position,c,reverse,higher,
                                                  trace,sorted(a.items()),sorted(b.items()),sorted(r.items()))))
                if final < score and (best is None or final < best[0]):
                    best = (final,a,b,r,(position,c,reverse,higher,trace))
    if best is None:
        break
    final,A,B,R,trace = best
    a,b,closed = certify(native, A, B, R)
    rounds.append((score,final,trace,sorted(A.items()),sorted(B.items()),sorted(R.items())))
    print(f'accepted_round={len(rounds)} residual_magnitude={score}->{final} closed={closed}', flush=True)
    print('placement_changed='+str(normalize(B)[0] != normalize(state['B0'])[0]), flush=True)
    if closed:
        break
a,b,closed = certify(native, A, B, R)
report = ['# Combined intermediate-rewrite and two-sided return', '',
          'Parent: the certified 186-term source correction.',
          'Each observed residual unit is rewritten without changing its value. '
          'Partial collection precedes cancellation. Both shift and cancellation orders are retained.',
          'Every tested endpoint expands and normalizes to the complete source dictionary. '
          'Every accepted endpoint additionally passes native Gödel.',
          f'Checked paths: {checked}; accepted rounds: {len(rounds)}.',
          f'Final correction magnitude: {magnitude(R)}; proper product closure: {closed}.', '',
          'Complete accepted paths: '+repr(rounds), '',
          'Complete comparison table: '+repr(comparisons), '',
          'Final operand A: '+repr(sorted(A.items())),
          'Final operand B: '+repr(sorted(B.items())),
          'Final correction: '+repr(sorted(R.items())), '']
for label,terms in [('operand A',A),('operand B',B),('correction',R)]:
    sign, encoded = state['magnitude_word'](terms)
    value = state['field'](state['call']('decode', encoded), 'value')
    reading = subprocess.run([str(ROOT/'run_cmds.sh'), 'tfactor read '+value],
                             cwd=ROOT, text=True, capture_output=True, check=True)
    logs.append(label+'\n'+reading.stdout)
    clean = re.sub(r'\x1b\[[0-9;]*m', '', reading.stdout)
    hex_word = next(line.split(':',1)[1].strip() for line in clean.splitlines()
                    if 'hex-digit word' in line)
    report.extend([f'{label} sign {sign}: `{value}`.', hex_word, ''])
(HERE/'combined_zero_partial_return.md').write_text('\n'.join(report)+'\n')
with gzip.open(HERE/'combined_zero_partial_return.log.gz','wt') as out:
    out.write('\n'.join(logs))
print(f'checked={checked} accepted={len(rounds)} final_residual={magnitude(R)} closed={closed}', flush=True)
