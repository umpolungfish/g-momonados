"""Retain actual and padded frame boundaries while reducing source gaps."""
from godel_markdown_records import read_record, write_record
import json
import math
from pathlib import Path

NAMES = ('sum', 'alternating_sum', 'product', 'sum_of_squares', 'first_minus_last')


def reductions(values):
    return (sum(values), sum(a if i % 2 == 0 else -a for i, a in enumerate(values)),
            math.prod(values), sum(a*a for a in values), values[0]-values[-1])


def probe(n):
    counts = {view: {op: dict(tested=0, unit=0, wholeSource=0, proper=0)
                     for op in NAMES} for view in ('actual', 'padded')}
    hits = []
    verified = 0
    for w in range(1, n.bit_length()+1):
        R = (1 << w)-1
        for s in range(w):
            tail = n >> s
            frames, masks = [], []
            while tail:
                masks.append((1 << min(w, tail.bit_length()))-1)
                frames.append(tail & R)
                tail >>= w
            padded = [R-a for a in frames]
            actual = [r-a for r, a in zip(masks, frames)]
            assert sum(padded) == len(frames)*R-sum(frames)
            assert sum(a*a for a in padded) == len(frames)*R*R-2*R*sum(frames)+sum(a*a for a in frames)
            assert sum(actual) == sum(masks)-sum(frames)
            alternating = sum(a if i % 2 == 0 else -a for i, a in enumerate(frames))
            assert sum(a if i % 2 == 0 else -a for i, a in enumerate(padded)) == (R if len(frames) % 2 else 0)-alternating
            assert padded[0]-padded[-1] == -(frames[0]-frames[-1])
            assert actual[0]-actual[-1] == masks[0]-masks[-1]-(frames[0]-frames[-1])
            # Actual final width is fixed by source positions, not payload occupancy.
            # All non-final frames use the complete mask even if locally sparse.
            assert masks[-1].bit_length() == (n.bit_length()-s-1) % w+1
            verified += 1
            for view, values in (('actual', actual), ('padded', padded)):
                for op, value in zip(NAMES, reductions(values)):
                    g = math.gcd(n, value)
                    bucket = counts[view][op]
                    bucket['tested'] += 1
                    category = 'unit' if g == 1 else 'wholeSource' if g == n else 'proper'
                    bucket[category] += 1
                    if category == 'proper':
                        hits.append(dict(width=w, shift=s, view=view, operation=op,
                                         result=str(value), factor=str(g), cofactor=str(n//g)))
    return dict(source=str(n), counts=counts, hits=hits, boundaryIdentitiesVerified=verified,
                evidence='Gap complements are value shapes, not falsity evidence. No proper hit leaves extraction None.')


if __name__ == '__main__':
    root = Path(__file__).parent
    for label, n in [('200', 1199215083673205158356661431646321119743288893586546308999201),
                     ('rsa100', 1522605027922533360535618378132637429718068114961380688657908494580122963258952897654000350692006139)]:
        report = probe(n)
        write_record(root / f'godel_shifted_gap_operations_{label}', report)
        print(json.dumps(dict(label=label, identities=report['boundaryIdentitiesVerified'],
                             properHits=len(report['hits']), counts=report['counts'])))
