"""Check the complete source return for the observed displacement-12 placement."""
from pathlib import Path
import subprocess
import re
import gzip
import argparse
import ast

parser = argparse.ArgumentParser()
parser.add_argument('--overlap-low-pole', action='store_true')
options = parser.parse_args()

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
logs = []


def call(*args):
    run = subprocess.run([str(ROOT/'godel'), *args], cwd=ROOT,
                         text=True, capture_output=True, check=True)
    logs.append(repr(args)+'\n'+run.stdout)
    return run.stdout


def field(text, name):
    return next(line.split(None, 1)[1] for line in text.splitlines()
                if re.match('^'+re.escape(name)+r'\s', line))


def word(positions):
    positions = set(positions)
    return '⊢'+''.join(FILLED if i in positions else EMPTY
                       for i in range(max(positions, default=0)+1))+'⊙⊡⊣'


def frame(left, operation, right):
    lc, rc = left[1:-3], right[1:-3]
    width = max(2, len(lc)//5, len(rc)//5)
    carrier = field(call('decode', '⊢'+lc+EMPTY*(2*width-len(lc)//5)+rc+'⊙⊡⊣'), 'value')
    return call('frame-op', carrier, str(width), '0', operation, str(2*width), '1')


record = HERE/('overlap_combined_return.md' if options.overlap_low_pole else
              'translated_signed_collection_single_polarity_hex_opposite_continued_e5b3df8bd92d.md')
text = record.read_text()
source = field(call('encode', (HERE/'source.txt').read_text().strip()), 'word')
if options.overlap_low_pole:
    displacement = int(re.search(r'displacement: ([0-9]+)',text).group(1))
    a,b,r = (dict(ast.literal_eval(re.search(label+r': (\[.*\])',text).group(1)))
             for label in ('Final operand A','Final operand B','Final correction'))
    assert min(a)==0 and abs(a[0])==1 and min(b)>0 and min(r)==0
    b[0] = r[0]*a[0]
    assert all(abs(c)==1 for c in b.values())
    positive = word(i for i,c in b.items() if c>0)
    negative = word(i for i,c in b.items() if c<0)
    greater,lesser = (positive,negative) if b[max(b)]>0 else (negative,positive)
    placement = field(call('encode',field(frame(greater,'sub',lesser),'result')),'word')
    assert 'PASS' in call('check','add',lesser,placement,greater)
else:
    displacement = int(re.search(r'Displacement: ([0-9]+)', text).group(1))
    assert 'Translation polarity: -1.' in text
    # The observed signed placement is U_0-U_d; take its positive magnitude.
    placement = word(range(displacement))
division = frame(source, 'divmod', placement)
quotient = field(call('encode', field(division, 'result')), 'word')
remainder = field(call('encode', field(division, 'remainder')), 'word')
product = field(call('encode', field(frame(placement, 'mul', quotient), 'result')), 'word')
assert 'PASS' in call('check', 'mul', placement, quotient, product)
assert 'PASS' in call('check', 'add', product, remainder, source)
reading_values = [('placement magnitude', placement), ('cofactor', quotient), ('retained remainder', remainder)]
report = ['# Complete return of the observed placement', '',
          f'Input record: {record.name}.', f'Displacement: {displacement}.', '',
          'Gödel independently verifies N = placement * quotient + remainder.',
          f'Exact placement return: {remainder == word([])}.', '',
          'The quotient is arithmetic for this already observed placement. '
          'No alternative unknown factor support is introduced.']
for label, encoded in reading_values:
    value = field(call('decode', encoded), 'value')
    run = subprocess.run([str(ROOT/'run_cmds.sh'), 'tfactor read '+value],
                         cwd=ROOT, text=True, capture_output=True, check=True)
    logs.append(label+'\n'+run.stdout)
    clean = re.sub(r'\x1b\[[0-9;]*m', '', run.stdout)
    hex_word = next(line.split(':', 1)[1].strip() for line in clean.splitlines()
                    if 'hex-digit word' in line)
    report.extend(['', label+': `'+value+'`.', '', hex_word])
report.extend(['', 'A nonzero remainder requires changing this placement; '
               'further payload rewrites with the same placement cannot close the source product.'])
record_name = 'placement_full_return'+('_overlap_low_pole' if options.overlap_low_pole else '')
(HERE/(record_name+'.md')).write_text('\n'.join(report)+'\n')
with gzip.open(HERE/(record_name+'.log.gz'), 'wt') as out:
    out.write('\n'.join(logs))
print('displacement='+str(displacement)+' remainder='+field(division, 'remainder')+
      ' full_source_equation=PASS exact_placement_return='+str(remainder == word([])), flush=True)
