"""Transport source carry locally into a nonadjacent signed word."""
from pathlib import Path
import re
import subprocess
import gzip

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
text = (HERE/'run_payload_collection.md').read_text()
terms = {}
for label, sign in [('Positive', 1), ('Negative', -1)]:
    positions = re.search(label+r' unit positions: ([0-9, ]+)',text).group(1)
    terms.update({int(i):sign for i in positions.split(', ')})
initial = len(terms)
trace = []
position = 0
while position <= max(terms, default=0):
    incoming = terms.get(position,0)
    if incoming % 2 == 0:
        output = 0
    else:
        output = 1 if (incoming+2*terms.get(position+1,0)) % 4 == 1 else -1
    carry = (incoming-output)//2
    if output:
        terms[position] = output
    else:
        terms.pop(position,None)
    if carry:
        terms[position+1] = terms.get(position+1,0)+carry
        if not terms[position+1]:
            del terms[position+1]
        trace.append((position,incoming,output,carry))
    position += 1
    assert position < 1000, 'carry transport escaped source width'
assert all(abs(c)==1 for c in terms.values())
assert all(i+1 not in terms for i in terms)
EMPTY,FILLED='≻⋈∈⊤∋','≻⋈∈⊥∋'


def word(positions):
    positions=set(positions)
    return '⊢'+''.join(FILLED if i in positions else EMPTY
                       for i in range(max(positions,default=0)+1))+'⊙⊡⊣'


source=(HERE/'source.txt').read_text().strip()
encoded=subprocess.run([str(ROOT/'godel'),'encode',source],cwd=ROOT,
                       text=True,capture_output=True,check=True)
native=next(line.split(None,1)[1] for line in encoded.stdout.splitlines()
            if line.startswith('word '))
positive=sorted(i for i,c in terms.items() if c>0)
negative=sorted(i for i,c in terms.items() if c<0)
check=subprocess.run([str(ROOT/'godel'),'check','add',native,word(negative),word(positive)],
                     cwd=ROOT,text=True,capture_output=True,check=True)
assert 'PASS' in check.stdout and 'FAIL' not in check.stdout
report=['# Complete source signed carry transport','',
        'Each local step uses the exact identity `c*U_i = d*U_i + '
        '((c-d)/2)*U_(i+1)`. The next occupied slot selects d in {-1,0,1} '
        'so the transported signed word has no adjacent occupied terms. '
        'There are no unknown factor cells.','',
        f'Signed terms: {initial} -> {len(terms)}.',
        f'Nonzero carry transports: {len(trace)}.','',
        'Positive unit positions: '+', '.join(map(str,positive)),'',
        'Negative unit positions: '+', '.join(map(str,negative)),'',
        'Transport trace (position, incoming, output, carry): '+repr(trace),'',
        'Gödel independently verifies the complete equation '
        '`source + negative-word = positive-word`. This is a transported '
        'source representation, not a factor certificate.']
(HERE/'signed_carry_transport.md').write_text('\n'.join(report)+'\n')
with gzip.open(HERE/'signed_carry_transport.log.gz','wt') as out:
    out.write(encoded.stdout+'\n'+check.stdout)
print(f'signed_terms={initial}->{len(terms)} transports={len(trace)} '
      'nonadjacent=PASS full_source_equation=PASS',flush=True)
