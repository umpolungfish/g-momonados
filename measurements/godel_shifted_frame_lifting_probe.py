"""Compose shared-payload lower-frame returns with shifted upper-frame lifting."""
from godel_markdown_records import read_record, write_record
import json
import argparse
from pathlib import Path
from godel_run_tiling_probe import probe as run_tiling_probe


def shared_payload_returns(lower):
    found = set()
    for width in range(1, lower.bit_length()+1):
        tail, index, payload, placement = lower, 0, None, 0
        while tail:
            frame = tail & ((1 << width)-1)
            if frame:
                if payload is None:
                    payload = frame
                elif frame != payload:
                    break
                placement += 1 << (index*width)
            tail >>= width
            index += 1
        else:
            if payload is not None:
                assert payload*placement == lower
                found.add((payload, placement))
    return found


def structural_returns(lower):
    found = shared_payload_returns(lower)
    for returned in run_tiling_probe(lower)['returns']:
        found.add(tuple(map(int, returned['factors'])))
    for width in range(1, lower.bit_length()+1):
        B = 1 << width
        tail, index, total, alternating, fold, alternate_fold, term = lower, 0, 0, 0, 0, 0, 1
        while tail:
            frame = tail & (B-1)
            total += frame
            alternating += frame if index % 2 == 0 else -frame
            if index:
                alternate_fold += frame*term
                term = B*term + (-1 if index % 2 else 1)
            tail >>= width
            fold += tail
            index += 1
        if total == B-1:
            pair = (B-1, fold+1)
            assert pair[0]*pair[1] == lower
            found.add(pair)
        if alternating == 0:
            pair = (B+1, alternate_fold)
            assert pair[0]*pair[1] == lower
            found.add(pair)
    return found


def probe(n, lower_operation=shared_payload_returns, subtract=False):
    counts = dict(tested=0, positiveSumFailed=0, sumBoundFailed=0,
                  differenceSquareBoundFailed=0, survivingShapes=0, closed=0)
    hits = []
    for width in range(1, n.bit_length()):
        lower = n & ((1 << width)-1)
        upper = n >> width
        if lower == 0:
            continue
        returned = None
        for shift in range(width):
            counts['tested'] += 1
            offset = upper << shift
            required_sum = offset-(1 << (width-shift)) if subtract else (1 << (width-shift))-offset
            if required_sum < 2:
                counts['positiveSumFailed'] += 1
                continue
            if required_sum > lower+1:
                counts['sumBoundFailed'] += 1
                continue
            if required_sum**2 < 4*lower:
                counts['differenceSquareBoundFailed'] += 1
                continue
            counts['survivingShapes'] += 1
            if returned is None:
                returned = lower_operation(lower)
            for a, b in returned:
                if a+b != required_sum:
                    continue
                p, q = (offset-a, offset-b) if subtract else (a+offset, b+offset)
                assert p*q == n and p > 1 and q > 1
                counts['closed'] += 1
                hits.append(dict(width=width, upperShift=shift, lower=str(lower), upper=str(upper),
                                 lowerReturn=[str(a),str(b)], offset=str(offset),
                                 factorReturn=[str(p),str(q)]))
    return dict(source=str(n), counts=counts, hits=hits,
                evidence='True for each closed return; None for surviving shapes without a returned lower pair')


def unequal_lift(n):
    hits, tested, positive, matches = [], 0, 0, 0
    for width in range(1, n.bit_length()):
        D = 1 << width
        lower, upper = n & (D-1), n >> width
        if not lower:
            continue
        pairs = structural_returns(lower)
        pairs |= {(b,a) for a,b in pairs}
        for a,b in sorted(pairs):
            for r in range(width):
                tested += 1
                residual = D-(b << r)
                if residual <= 0:
                    break
                positive += 1
                word = a+(upper << r)
                residual_zeros = (residual & -residual).bit_length()-1
                word_zeros = (word & -word).bit_length()-1
                t = residual_zeros-word_zeros
                if t < 0 or residual != word << t:
                    continue
                p,q = word,b+(upper << t)
                assert p*q == n and p > 1 and q > 1
                if n % 2:
                    assert r == t
                matches += 1
                hits.append(dict(width=width, lowerReturn=[str(a),str(b)],
                                 upper=str(upper), shifts=[r,t], residual=str(residual),
                                 factorReturn=[str(p),str(q)]))
    return dict(source=str(n), addressedShifts=tested, positiveResiduals=positive,
                closed=matches,hits=hits,
                extractionEvidence='True' if hits else 'None')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--composed', action='store_true')
    parser.add_argument('--unequal', action='store_true')
    parser.add_argument('--subtract', action='store_true')
    args = parser.parse_args()
    sources = [21,35,91,143,221,8051,
               1199215083673205158356661431646321119743288893586546308999201,
               1522605027922533360535618378132637429718068114961380688657908494580122963258952897654000350692006139]
    if args.unequal:
        reports = [unequal_lift(n) for n in sources]
        output = dict(operation='a*b=L; n=L+2^w*H; emit (a+2^r*H)*(b+2^t*H) when 2^w-b*2^r=(a+H*2^r)*2^t',
                      selector='Compare odd parts and initial empty-cell lengths of the residual and returned word',
                      reports=reports, noFactorWidthAssumption=True)
        write_record(Path(__file__).with_name('godel_unequal_frame_lifting'), output)
        print(json.dumps([dict(source=x['source'],closed=x['closed'],hits=x['hits'][:4]) for x in reports]))
        raise SystemExit
    reports = [probe(n,structural_returns if args.composed or args.subtract else shared_payload_returns, args.subtract) for n in sources]
    operation = 'n=L+2^w*H; a*b=L; c=2^r*H; if c-a-b=2^(w-r), emit (c-a)*(c-b)' if args.subtract else 'n=L+2^w*H; a*b=L; c=2^r*H; if a+b+c=2^(w-r), emit (a+c)*(b+c)'
    output = dict(operation=operation,
                  lowerOperation='shared payload, filled sum, alternating balance' if args.composed or args.subtract else 'shared occupied frame payload and placement, including unit lower operands',
                  noFactorWidthAssumption=True, reports=reports)
    filename = 'godel_subtractive_frame_lifting' if args.subtract else 'godel_composed_frame_lifting' if args.composed else 'godel_shifted_frame_lifting'
    write_record(Path(__file__).with_name(filename), output)
    print(json.dumps([dict(source=x['source'], counts=x['counts'], hits=x['hits'][:4]) for x in reports]))
