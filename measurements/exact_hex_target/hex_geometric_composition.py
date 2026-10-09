"""Read complete repeated signed compositions at observed hex boundaries."""
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
text = (HERE / 'run_payload_collection.md').read_text()
source = {}
for label, sign in [('Positive', 1), ('Negative', -1)]:
    positions = re.search(label+r' unit positions: ([0-9, ]+)', text).group(1)
    source.update({int(i):sign for i in positions.split(', ')})

rows = []
for displacement in range(4, max(source)+1, 4):
    for polarity in (1, -1):
        # Chains are maximal observed successive translations. No absent
        # source support is proposed or tested as an unknown factor cell.
        starts = [i for i,c in source.items()
                  if source.get(i-displacement) != polarity*c]
        chains = []
        for start in sorted(starts):
            chain = [start]
            while source.get(chain[-1]+displacement) == polarity*source[chain[-1]]:
                chain.append(chain[-1]+displacement)
            chains.append(chain)
        assert sorted(i for chain in chains for i in chain) == sorted(source)
        maximum_copies = max(map(len, chains))
        for copies in range(2, maximum_copies+1):
            payload, used = {}, set()
            for chain in chains:
                for offset in range(0, len(chain)-copies+1, copies):
                    group = chain[offset:offset+copies]
                    payload[group[0]] = source[group[0]]
                    used.update(group)
            residual = {i:c for i,c in source.items() if i not in used}
            reconstructed = dict(residual)
            for i,c in payload.items():
                sign = 1
                for index in range(copies):
                    position = i+index*displacement
                    assert position not in reconstructed
                    reconstructed[position] = sign*c
                    sign *= polarity
            assert reconstructed == source
            rows.append((len(residual), displacement, polarity, copies, len(payload)))

rows.sort()
closed = [row for row in rows if row[0] == 0]
large = [row for row in rows if row[1]*(row[3]-1)+1 >= 100]
report = ['# Complete signed hex-composition census', '',
          'This census considers observed maximal translated chains at '
          'whole-hex boundaries, with equal or alternating signs. It collects '
          'complete repeated groups and preserves every uncollected source '
          'term. No unknown factor support, divisibility test, or solver '
          'participates.', '',
          f'Complete source signed terms: {len(source)}.',
          f'Observed repeated compositions: {len(rows)}.',
          f'Compositions with zero residual: {len(closed)}.', '',
          'Every retained symbolic expansion matches the complete signed '
          'source term dictionary exactly.', '',
          '| Residual terms | Displacement cells | Polarity | Copies | Payload terms |',
          '| --- | --- | --- | --- | --- |']
report += ['| '+' | '.join(map(str,row))+' |' for row in rows]
report += ['', 'Best composition with cofactor width at least 100 cells: '
           + repr(min(large) if large else None), '',
           'The census rules out only this literal repeated-composition '
           'family on the present signed representation. It does not rule '
           'out a product exposed by another source-preserving carry '
           'transport. It supplies no proper-factor certificate.']
(HERE/'hex_geometric_composition.md').write_text('\n'.join(report)+'\n')
print(f'compositions={len(rows)} closed={len(closed)} best={rows[0]} '
      f'best_large={min(large) if large else None}', flush=True)
