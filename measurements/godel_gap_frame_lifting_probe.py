"""Compose a lower gap-plus-unit return with mixed addition/subtraction lifting."""
from godel_markdown_records import read_record, write_record
import json
from pathlib import Path
from godel_shifted_frame_lifting_probe import structural_returns


def probe(n, lower_operation=structural_returns):
    counts = dict(tested=0, differenceBoundFailed=0, survivingShapes=0, closed=0, unitReturns=0)
    hits = []
    for width in range(1, n.bit_length()):
        D = 1 << width
        lower, upper = n & (D-1), n >> width
        gap_plus_unit = D-lower
        returned = None
        for shift in range(width):
            counts['tested'] += 1
            c = (upper+1) << shift
            required_difference = (1 << (width-shift))-c
            if abs(required_difference) > gap_plus_unit-1:
                counts['differenceBoundFailed'] += 1
                continue
            counts['survivingShapes'] += 1
            if returned is None:
                returned = lower_operation(gap_plus_unit)
                returned |= {(b,a) for a,b in returned}
            for a,b in sorted(returned):
                if a-b != required_difference:
                    continue
                p,q = c+a,c-b
                assert p*q == n and q > 0
                if q == 1:
                    counts['unitReturns'] += 1
                    continue
                counts['closed'] += 1
                hits.append(dict(width=width, upperShift=shift, lower=str(lower), upper=str(upper),
                                 gapPlusUnit=str(gap_plus_unit), lowerGapReturn=[str(a),str(b)],
                                 offset=str(c), factorReturn=[str(p),str(q)]))
    return dict(source=str(n), counts=counts, hits=hits,
                factorEvidence='True' if hits else 'None')


if __name__ == '__main__':
    sources = [21,35,91,143,221,8051,
               1199215083673205158356661431646321119743288893586546308999201,
               1522605027922533360535618378132637429718068114961380688657908494580122963258952897654000350692006139]
    reports = [probe(n) for n in sources]
    output = dict(operation='n=2^w*(H+1)-M; M=2^w-L=a*b; c=2^r*(H+1); if c+a-b=2^(w-r), emit (c+a)*(c-b)',
                  lowerOperations=['shared payload placement', 'filled frame sum', 'alternating balance'],
                  noFactorWidthAssumption=True, reports=reports)
    write_record(Path(__file__).with_name('godel_gap_frame_lifting'), output)
    print(json.dumps([dict(source=x['source'],counts=x['counts'],hits=x['hits'][:3]) for x in reports]))
