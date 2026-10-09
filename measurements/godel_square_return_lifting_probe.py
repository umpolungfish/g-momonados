"""Compose the authorized square-sum gcd return with source-frame lifts."""
from godel_markdown_records import read_record, write_record
import json
import math
import time
from functools import lru_cache
from pathlib import Path
from godel_shifted_frame_lifting_probe import probe as payload_lift, structural_returns
from godel_gap_frame_lifting_probe import probe as gap_lift

metrics = dict(lowerSources=0, squareSumTests=0, properSquareReturns=0)


@lru_cache(maxsize=None)
def lower_returns(n):
    metrics['lowerSources'] += 1
    result = structural_returns(n)
    for width in range(1, n.bit_length()+1):
        mask = (1 << width)-1
        for shift in range(width):
            tail, squares = n >> shift, 0
            while tail:
                frame = tail & mask
                squares += frame*frame
                tail >>= width
            metrics['squareSumTests'] += 1
            g = math.gcd(squares,n)
            if 1 < g < n:
                metrics['properSquareReturns'] += 1
                assert n % g == 0
                result.add((g,n//g))
    return frozenset(result)


if __name__ == '__main__':
    reports=[]
    for n in [1034010555,
              1199215083673205158356661431646321119743288893586546308999201,
              1522605027922533360535618378132637429718068114961380688657908494580122963258952897654000350692006139]:
        start=time.perf_counter()
        rows=dict(positive=payload_lift(n,lower_returns),
                  subtractive=payload_lift(n,lower_returns,True),
                  gapMixed=gap_lift(n,lower_returns))
        reports.append(dict(source=str(n),routes=rows,elapsedSeconds=time.perf_counter()-start))
        print(json.dumps(dict(source=str(n),closed={k:v['counts']['closed'] for k,v in rows.items()},
                              elapsedSeconds=reports[-1]['elapsedSeconds'],metrics=metrics)),flush=True)
    output=dict(lowerOperation='structural returns plus all shifted sum-of-squares gcd returns',
                metrics=metrics,reports=reports,noFactorWidthAssumption=True)
    write_record(Path(__file__).with_name('godel_square_return_lifting'), output)
