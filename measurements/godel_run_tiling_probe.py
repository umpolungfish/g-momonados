"""Use analyzer run starts as local frame origins; return exact placement words."""
from godel_markdown_records import read_record, write_record
import json
from pathlib import Path


def intervals(n, occupied):
    result, start = [], None
    for i in range(n.bit_length()+1):
        matches = i < n.bit_length() and bool(n >> i & 1) == occupied
        if matches and start is None:
            start = i
        if not matches and start is not None:
            result.append((start,i))
            start = None
    return result


def placement(runs, width):
    return sum(1 << start for lo,hi in runs for start in range(lo,hi,width))


def probe(n):
    runs, gaps = intervals(n,True), intervals(n,False)
    returns = []
    for d in range(2,n.bit_length()+1):
        R = (1 << d)-1
        if all((hi-lo) % d == 0 for lo,hi in runs):
            Q = placement(runs,d)
            assert R*Q == n
            if Q > 1:
                returns.append(dict(view='occupied runs',width=d,factors=[str(R),str(Q)]))
        if n.bit_length() % d == 0 and all((hi-lo) % d == 0 for lo,hi in gaps):
            full = sum(1 << i for i in range(0,n.bit_length(),d))
            missing = placement(gaps,d)
            assert missing*((1 << d)-1) == ((1 << n.bit_length())-1)-n
            assert full >= missing
            Q = full-missing
            assert R*Q == n
            if Q > 1:
                returns.append(dict(view='gap runs and retained full width',width=d,factors=[str(R),str(Q)]))
    return dict(source=str(n),runs=runs,gaps=gaps,returns=returns,
                factorEvidence='True' if returns else 'None')


if __name__ == '__main__':
    sources = [123,819,21,35,91,143,221,8051,
               1199215083673205158356661431646321119743288893586546308999201,
               1522605027922533360535618378132637429718068114961380688657908494580122963258952897654000350692006139]
    reports = [probe(n) for n in sources]
    checks = returns = 0
    for n in range(2,4096):
        row = probe(n)
        checks += 1
        returns += len(row['returns'])
    output = dict(operations=['tile occupied runs from each own start','tile gaps then subtract their placement word'],
                  reports=reports,sourceOnlyVerificationCount=checks,verifiedProperReturns=returns)
    write_record(Path(__file__).with_name('godel_run_tiling_relationships'), output)
    print(json.dumps(dict(reports=reports,sourceOnlyVerificationCount=checks,verifiedProperReturns=returns)))
