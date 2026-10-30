"""Construct factor candidates from phase prefixes and exact integer bounds.

Run: python3 phase_prefix.py N WIDTH --radix-bits 1
Controls: python3 phase_prefix.py --verify
Width/radix sweep: python3 phase_prefix.py --benchmark
Joint high parts: add --joint to a single input or the width/radix sweep.
Full product bounds: use --curve for an exact rational strip around Q=N/P.
The search covers odd pairs with both factors WIDTH bits, P <= Q.
It constructs a prefix tree without a truth table. All work is counted,
including rejected prefixes; exhaustive completion is explicit in JSON.
"""

import argparse
import json
from math import isqrt
from random import Random
from time import perf_counter


def tighten(n, p, q, step, bounds, stats):
    """Apply four sound contraction rounds; count work inside each prefix too."""
    pl, ph, ql, qh = bounds
    for iteration in range(4):
        stats["bound_rounds"] += 1
        old = pl, ph, ql, qh
        pl += (p - pl) % step
        ph -= (ph - p) % step
        ql += (q - ql) % step
        qh -= (qh - q) % step
        if pl > ph or ql > qh or pl * ql > n or ph * qh < n:
            return None
        if iteration == 3:
            return pl, ph, ql, qh
        pl = max(pl, (n + qh - 1) // qh)
        ph = min(ph, n // ql, qh)
        ql = max(ql, (n + ph - 1) // ph, pl)
        qh = min(qh, n // pl)
        if (pl, ph, ql, qh) == old:
            return old


def floor_sum(count, modulus, slope, offset, stats):
    """Sum floor((slope*i+offset)/modulus), using Euclidean reduction."""
    total = 0
    while True:
        stats["floor_sum_steps"] += 1
        quotient, slope = divmod(slope, modulus)
        total += quotient * count * (count-1) // 2
        quotient, offset = divmod(offset, modulus)
        total += quotient * count
        top = slope * count + offset
        if top < modulus:
            return total
        count, offset = divmod(top, modulus)
        modulus, slope = slope, modulus


def joint_candidates(n, p, q, step, box, stats, limit=8):
    """Return all joint candidates when sparse, [] if empty, None if still dense.

    P=p+step*A, Q=q+step*B gives q*A+p*B+step*A*B=H.
    Reducing modulo step gives a modular line in the high-part rectangle.
    Floor sums count its points without enumerating either coordinate.
    """
    p, q = p % step, q % step
    pl, ph, ql, qh = box
    al, ah = (pl-p)//step, (ph-p)//step
    bl, bh = (ql-q)//step, (qh-q)//step
    na, nb = ah-al+1, bh-bl+1
    # Both coefficients are odd, so each complete period in either coordinate
    # contributes a point for every value of the other coordinate.
    if max(na * (nb//step), nb * (na//step)) > limit:
        stats["joint_dense_skips"] += 1
        return None
    stats["joint_checks"] += 1
    h = (n-p*q)//step
    inverse = pow(p, -1, step)
    slope, offset = (-q*inverse) % step, (h*inverse) % step

    def count(left, right):
        stats["lattice_counts"] += 1
        length = right-left+1
        shift = offset+slope*left
        return (floor_sum(length, step, -slope, bh-shift, stats)
                - floor_sum(length, step, -slope, bl-1-shift, stats))

    number = count(al, ah)
    if number > limit:
        return None
    if number == 0:
        stats["joint_rejected"] += 1
        return []
    stats["joint_terminal"] += 1
    candidates = []
    pending = [(al, ah, number)]
    while pending:
        left, right, amount = pending.pop()
        if left == right:
            first = bl + (offset+slope*left-bl) % step
            for b in range(first, bh+1, step):
                candidates.append((p+step*left, q+step*b))
            continue
        mid = (left+right)//2
        lower = count(left, mid)
        if lower:
            pending.append((left, mid, lower))
        if amount-lower:
            pending.append((mid+1, right, amount-lower))
    assert len(candidates) == number
    return candidates


def curve_strip(n, p, q, step, box):
    """Integer coefficients of the tangent/secant strip in modular-line coordinates."""
    p, q = p % step, q % step
    pl, ph, ql, qh = box
    length = (ph-pl)//step+1
    h = (n-p*q)//step
    inverse = pow(p, -1, step)
    slope = (-q*inverse) % step
    offset = (h*inverse) % step
    a0 = (pl-p)//step
    qbase = q+step*(offset+slope*a0)
    center = pl+step*(length//2)
    upper_den, lower_den = pl*ph, center*center
    upper_a = -n*step-upper_den*step*slope
    upper_b = n*ph-upper_den*qbase
    lower_a = -n*step-lower_den*step*slope
    lower_b = n*(2*center-pl)-lower_den*qbase
    upper_mod, lower_mod = upper_den*step*step, lower_den*step*step

    return upper_mod, upper_a, upper_b, lower_mod, lower_a, lower_b


def strip_count(strip, left, right, stats):
    stats["curve_counts"] += 1
    upper_mod, upper_a, upper_b, lower_mod, lower_a, lower_b = strip
    size = right-left+1
    return (floor_sum(size, upper_mod, upper_a, upper_b+upper_a*left, stats)
            + floor_sum(size, lower_mod, -lower_a, -lower_b-lower_a*left, stats)
            + size)


def curve_candidates(n, p, q, step, box, stats, limit=8, segments=1):
    """Count the modular line inside exact rational bounds on Q=N/P.

    The secant is an upper bound and the midpoint tangent a lower bound.
    Both bounds stay fixed during candidate bisection so counts partition.
    """
    stats["curve_checks"] += 1
    p, q = p % step, q % step
    pl, ph, ql, qh = box
    length = (ph-pl)//step+1
    pieces = []
    number = 0
    segments = min(segments, length)
    for segment in range(segments):
        first, end = length*segment//segments, length*(segment+1)//segments
        start_p = pl+step*first
        strip = curve_strip(n, p, q, step, (start_p, pl+step*(end-1), ql, qh))
        stats["curve_pieces"] += 1
        amount = strip_count(strip, 0, end-first-1, stats)
        assert amount >= 0
        number += amount
        if number > limit:
            return None
        if amount:
            pieces.append((start_p, strip, end-first, amount))
    if number == 0:
        stats["curve_rejected"] += 1
        return []
    stats["curve_terminal"] += 1
    candidates = []
    for start_p, strip, size, amount in pieces:
        pending = [(0, size-1, amount)]
        while pending:
            left, right, amount = pending.pop()
            if left == right:
                pp = start_p+step*left
                stats["curve_division_tests"] += 1
                qq, remainder = divmod(n, pp)
                if remainder == 0 and ql <= qq <= qh and qq % step == q:
                    candidates.append((pp, qq))
                continue
            mid = (left+right)//2
            lower = strip_count(strip, left, mid, stats)
            assert 0 <= lower <= amount
            if lower:
                pending.append((left, mid, lower))
            if amount-lower:
                pending.append((mid+1, right, amount-lower))
    return candidates


def solve(n, m, radix_bits=1, node_limit=None, joint=False, curve=False, curve_segments=1):
    if n < 1 or not n & 1 or m < 3 or not 1 <= radix_bits <= 8:
        raise ValueError("N must be positive and odd, width >= 3, radix bits in 1..8")
    if node_limit is not None and node_limit < 1:
        raise ValueError("node limit must be positive")
    if not 1 <= curve_segments <= 64:
        raise ValueError("curve segments must be in 1..64")
    start = perf_counter()
    mod = 1 << m
    lo, hi = mod >> 1, mod - 1
    stats = dict(n=n, width=m, radix_bits=radix_bits, joint=joint or curve, curve=curve,
                 curve_segments=curve_segments, nodes=0,
                 rejected=0, singleton_tests=0, expanded=0, max_pending=0, bound_rounds=0,
                 orbit_positions=mod >> 1, per_depth={}, pairs=[], complete=True)
    stats.update(joint_checks=0, joint_dense_skips=0, joint_rejected=0,
                 joint_terminal=0, lattice_counts=0, floor_sum_steps=0,
                 joint_product_tests=0)
    stats.update(curve_checks=0, curve_counts=0, curve_rejected=0,
                 curve_terminal=0, curve_division_tests=0, curve_pieces=0)
    if not lo * lo <= n <= hi * hi:
        stats["seconds"] = perf_counter() - start
        return stats
    bounds = max(lo, (n + hi - 1) // hi), min(hi, isqrt(n)), lo, hi
    # A state fixes k mod 2^(t-3). Remaining k bits cannot change P or Q mod 2^t.
    stack = [(3, s % mod, n * pow(s, -1, mod) % mod, bounds)
             for s in (1, 3, -1, -3)]
    jumps = {}
    pairs = set()
    while stack:
        if node_limit is not None and stats["nodes"] >= node_limit:
            stats["complete"] = False
            break
        stats["max_pending"] = max(stats["max_pending"], len(stack))
        t, p, q, box = stack.pop()
        stats["nodes"] += 1
        stats["per_depth"][t] = stats["per_depth"].get(t, 0) + 1
        box = tighten(n, p, q, 1 << t, box, stats)
        if box is None:
            stats["rejected"] += 1
            continue
        pl, ph, ql, qh = box
        if pl == ph or ql == qh:
            stats["singleton_tests"] += 1
            a = pl if pl == ph else ql
            b, remainder = divmod(n, a)
            if remainder == 0 and lo <= b <= hi:
                pp, qq = sorted((a, b))
                if pl <= pp <= ph and ql <= qq <= qh and (pp-p) % (1 << t) == 0 and (qq-q) % (1 << t) == 0:
                    pairs.add((pp, qq))
            continue
        if joint or curve:
            if curve:
                candidates = curve_candidates(n, p, q, 1 << t, box, stats, segments=curve_segments)
            else:
                candidates = joint_candidates(n, p, q, 1 << t, box, stats)
            if candidates is not None:
                for pp, qq in candidates:
                    stats["joint_product_tests"] += 1
                    if pp <= qq and pp*qq == n:
                        pairs.add((pp, qq))
                continue
        bits = min(radix_bits, m-t)
        assert bits > 0, "full-width residue must be a singleton"
        if t not in jumps:
            jump = pow(9, 1 << (t-3), mod)
            jumps[t] = jump, pow(jump, -1, mod)
        jump, inverse = jumps[t]
        stats["expanded"] += 1
        for _ in range(1 << bits):
            stack.append((t+bits, p, q, box))
            p, q = p * jump % mod, q * inverse % mod
    stats["pairs"] = sorted(pairs)
    stats["seconds"] = perf_counter() - start
    return stats


def exhaustive(n, m):
    lo, hi = 1 << (m-1), (1 << m)-1
    return [(p, n//p) for p in range(lo+1, min(hi, isqrt(n))+1, 2)
            if n % p == 0 and lo <= n//p <= hi]


def verify():
    rng = Random(20260910)
    cases = 0
    modes = [(False, False, 1), (True, False, 1)] + [
        (False, True, segments) for segments in (1, 2, 4, 8, 16, 64)]
    # All odd inputs around the complete product domain at small widths.
    # Includes primes, squares, many-factor composites and out-of-band products.
    for m in range(3, 7):
        for n in range(1, (1 << (2*m))+3, 2):
            expected = exhaustive(n, m)
            for radix in (1, 2, 3):
                for joint, curve, segments in modes:
                    got = solve(n, m, radix, joint=joint, curve=curve, curve_segments=segments)
                    assert got["complete"] and got["pairs"] == expected, (n, m, radix, got, expected)
                    cases += 1
    for m in range(7, 14):
        lo, hi = 1 << (m-1), (1 << m)-1
        for _ in range(30):
            p, q = rng.randrange(lo+1, hi+1, 2), rng.randrange(lo+1, hi+1, 2)
            for n in (p*q, p*q+2):
                expected = exhaustive(n, m)
                for radix in (1, 2, 3):
                    for joint, curve, segments in modes:
                        got = solve(n, m, radix, joint=joint, curve=curve, curve_segments=segments)
                        assert got["complete"] and got["pairs"] == expected, (n, m, radix, got, expected)
                        cases += 1
    from collections import defaultdict
    for _ in range(2000):
        length, modulus = rng.randrange(0, 50), rng.randrange(1, 100)
        slope, offset = rng.randrange(-200, 200), rng.randrange(-200, 200)
        assert floor_sum(length, modulus, slope, offset, defaultdict(int)) == sum(
            (slope*i+offset)//modulus for i in range(length))
        step = 1 << rng.randrange(1, 8)
        p, q = rng.randrange(1, step, 2), rng.randrange(1, step, 2)
        al, bl = rng.randrange(0, 20), rng.randrange(0, 20)
        ah, bh = al+rng.randrange(0, 15), bl+rng.randrange(0, 15)
        h = rng.randrange(0, 10000)
        n = p*q+step*h
        expected = [(p+step*a, q+step*b) for a in range(al, ah+1)
                    for b in range(bl, bh+1) if (q*a+p*b-h) % step == 0]
        got = joint_candidates(n, p, q, step,
                               (p+step*al, p+step*ah, q+step*bl, q+step*bh),
                               defaultdict(int))
        if len(expected) <= 8:
            assert got is not None and sorted(got) == expected, (n, step, got, expected)
        else:
            assert got is None
    from fractions import Fraction
    from math import ceil, floor
    for _ in range(1000):
        step = 1 << rng.randrange(1, 7)
        p, q = rng.randrange(1, step, 2), rng.randrange(1, step, 2)
        al = rng.randrange(1, 20)
        ah = al+rng.randrange(0, 20)
        pl, ph = p+step*al, p+step*ah
        n = p*q+step*rng.randrange(1, 10000)
        box = (pl, ph, q, q+step*100)
        center = pl+step*((ah-al+1)//2)
        expected = 0
        for a in range(al, ah+1):
            pp = p+step*a
            lower = Fraction(n*(2*center-pp), center*center)
            upper = Fraction(n*(pl+ph-pp), pl*ph)
            # Direct modular inversion at each P is independent of the high-part line.
            residue = n*pow(pp, -1, step*step) % (step*step)
            expected += (floor((upper-residue)/(step*step))
                         - ceil((lower-residue)/(step*step))+1)
        strip = curve_strip(n, p, q, step, box)
        got = strip_count(strip, 0, ah-al, defaultdict(int))
        assert got == expected, (n, step, box, got, expected)
    for joint, curve in ((False, False), (True, False), (False, True)):
        limited = solve(62615533, 13, node_limit=1, joint=joint, curve=curve)
        assert not limited["complete"]
    print(json.dumps(dict(verification="passed", exhaustive_comparisons=cases,
                          seed=20260910, incomplete_control="passed",
                          floor_sum_controls=2000, joint_rectangle_controls=2000,
                          rational_strip_controls=1000)))


def benchmark(joint=False, curve=False, curve_segments=1):
    def next_prime(value):
        candidate = value | 1
        while any(candidate % d == 0 for d in range(3, isqrt(candidate)+1, 2)):
            candidate += 2
        return candidate

    for m in (12, 16, 20, 24):
        lo = 1 << (m-1)
        for family, seeds in (("close", (lo+lo//3, lo+lo//3+100)),
                              ("separated", (lo+lo//16, lo+7*lo//8)),
                              ("near_floor", (lo+1, lo+lo//32))):
            p, q = map(next_prime, seeds)
            for radix in (1, 2, 3):
                result = solve(p*q, m, radix, node_limit=1000000, joint=joint,
                               curve=curve, curve_segments=curve_segments)
                result["family"] = family
                result["expected_pair"] = sorted((p, q))
                if result["complete"]:
                    assert result["pairs"] == [tuple(sorted((p, q)))], result
                else:
                    assert all(a*b == p*q for a, b in result["pairs"])
                print(json.dumps(result), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("n", type=int, nargs="?")
    parser.add_argument("width", type=int, nargs="?")
    parser.add_argument("--radix-bits", type=int, default=1)
    parser.add_argument("--node-limit", type=int)
    parser.add_argument("--verify", action="store_true")
    parser.add_argument("--benchmark", action="store_true")
    parser.add_argument("--joint", action="store_true",
                        help="count joint high-part constraints and directly test sparse bands")
    parser.add_argument("--curve", action="store_true",
                        help="bound the full product curve with exact rational tangent and secant")
    parser.add_argument("--curve-segments", type=int, default=1,
                        help="partition each curve band into 1..64 rational strips (default: 1)")
    args = parser.parse_args()
    if args.verify:
        verify()
    elif args.benchmark:
        benchmark(args.joint, args.curve, args.curve_segments)
    elif args.n is None or args.width is None:
        parser.error("provide N and WIDTH, or --verify")
    else:
        print(json.dumps(solve(args.n, args.width, args.radix_bits, args.node_limit,
                               args.joint, args.curve, args.curve_segments)))
