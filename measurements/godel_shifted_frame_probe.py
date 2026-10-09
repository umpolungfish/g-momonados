"""User-authorized shifted binary-frame reductions; exact integer evidence."""
from godel_markdown_records import read_record, write_record
import argparse
import json
import math
import time
from pathlib import Path


def probe(n):
    started = time.perf_counter()
    names = ('sum', 'alternating_sum', 'product', 'sum_of_squares', 'first_minus_last')
    counts = {name: dict(tested=0, unit=0, wholeSource=0, proper=0) for name in names}
    frame_counts = dict(tested=0, unit=0, wholeSource=0, proper=0, zero=0)
    hits = []
    frame_hits = []
    for width in range(1, n.bit_length() + 1):
        mask = (1 << width) - 1
        for shift in range(width):
            tail = n >> shift
            frames = []
            while tail:
                frames.append(tail & mask)
                tail >>= width
            for index, value in enumerate(frames):
                g = math.gcd(value, n)
                frame_counts['tested'] += 1
                frame_counts['zero'] += value == 0
                category = 'unit' if g == 1 else 'wholeSource' if g == n else 'proper'
                frame_counts[category] += 1
                if category == 'proper':
                    frame_hits.append(dict(width=width, shift=shift, index=index,
                                           frame=str(value), gcd=str(g)))
            results = (sum(frames), sum(a if i % 2 == 0 else -a for i, a in enumerate(frames)),
                       math.prod(frames), sum(a*a for a in frames), frames[0] - frames[-1])
            for name, value in zip(names, results):
                g = math.gcd(value, n)
                counts[name]['tested'] += 1
                category = 'unit' if g == 1 else 'wholeSource' if g == n else 'proper'
                counts[name][category] += 1
                if category == 'proper':
                    assert n % g == 0 and 1 < g < n
                    hits.append(dict(width=width, shift=shift, operation=name,
                                     result=str(value), gcd=str(g), cofactor=str(n // g)))
    return dict(source=str(n), sourceBitlength=n.bit_length(), widths=[1, n.bit_length()],
                shiftRule='0 <= s < w; discard prefix below s; high-zero-pad final frame',
                operations=list(names), counts=counts, hits=hits, individualFrames=frame_counts,
                individualFrameHits=frame_hits, elapsedSeconds=time.perf_counter()-started,
                evidence='No proper return leaves factor extraction None; zero and whole-source gcds are trivial.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('source', type=int)
    parser.add_argument('output', type=Path, help='Record name inside godel_relationships.md')
    args = parser.parse_args()
    if args.source < 2:
        parser.error('source must be at least two')
    result = probe(args.source)
    write_record(args.output, result)
    print(json.dumps({key: result[key] for key in ('sourceBitlength', 'counts', 'individualFrames', 'elapsedSeconds')}))
