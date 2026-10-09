"""Alternate local signed cancellation with literal payload absorption."""
from godel_markdown_records import read_record, write_record
from godel_symbolic_residual_absorption import absorb
from godel_symbolic_boundary_collect import value


def cancel(word, higher):
    t = dict(word)
    moves = []
    seen = set()
    while True:
        key = tuple(sorted(t.items()))
        assert key not in seen, 'Cancellation cycle'
        seen.add(key)
        found = False
        for i in sorted(t, reverse=higher):
            c = t.get(i, 0)
            if abs(c) >= 2:
                sign = 1 if c > 0 else -1
                t[i] -= 2*sign
                t[i+1] = t.get(i+1, 0)+sign
                moves.append(['merge', i, sign])
            elif c and c*t.get(i+1, 0) < 0:
                sign = 1 if t[i+1] > 0 else -1
                t[i+1] -= sign
                t[i] += 2*sign
                moves.append(['split-cancel', i, sign])
            else:
                continue
            t = {j:d for j,d in t.items() if d}
            found = True
            break
        if not found:
            assert value(t) == value(word)
            return t, moves


def main():
    prior = read_record('godel_symbolic_overlap_collections')
    reports = []
    for row in prior['reports']:
        n = int(row['source'])
        for index, state in enumerate(row['states']):
            for swap in (False, True):
                A, B = dict(state['payload']), dict(state['placement'])
                if swap:
                    A, B = B, A
                anchor = min(A)
                A = {i-anchor:c for i,c in A.items()}
                B = {i+anchor:c for i,c in B.items()}
                for higher in (False, True):
                    R = dict(state['correction'])
                    placement = dict(B)
                    rounds = []
                    while True:
                        R, rewrites = cancel(R, higher)
                        placement, R, copies = absorb(A, placement, R)
                        assert value(A)*value(placement)+value(R) == n
                        rounds.append(dict(rewrites=rewrites,absorptions=copies,
                            remainingCorrectionTerms=len(R)))
                        if not copies:
                            break
                    closed = not R and abs(value(A))>1 and abs(value(placement))>1
                    reports.append(dict(source=str(n),parentState=index,swap=swap,
                        rewriteOrder='higher-first' if higher else 'lower-first',
                        payload=sorted(A.items()),initialPlacement=sorted(B.items()),
                        rounds=rounds,placement=sorted(placement.items()),correction=sorted(R.items()),
                        exactReturnEvidence='True',sourceFactorEvidence='True' if closed else 'None'))
    write_record('godel_symbolic_cancel_collect',dict(reports=reports))
    print('States:',len(reports),'absorbing states:',sum(any(q['absorptions'] for q in r['rounds']) for r in reports))
    print('Whole returns:',sum(r['sourceFactorEvidence']=='True' for r in reports))
    for n in sorted(set(r['source'] for r in reports),key=int):
        rows=[r for r in reports if r['source']==n]
        print('Source:',n,'minimum correction terms:',min(len(r['correction']) for r in rows),
            'maximum absorption rounds:',max(sum(bool(q['absorptions']) for q in r['rounds']) for r in rows))


if __name__ == '__main__':
    main()
