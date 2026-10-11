"""Source-only experiments with the hosted Lean interfaces. No factor references."""
from pathlib import Path
import ast
import json
import re
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SOURCE = int((HERE / 'source.decimal').read_text())
WORD = (HERE / 'source.imasm').read_text().strip()
SEED = str(SOURCE & 15)
FRAME = str((SOURCE >> 4) & 15)
CASES = [[x] for x in ('identity', 'negation', 'hold')]
CASES += [[x, SEED] for x in ('seeded', 'constant', 'Y', 'join_i', 'meet_i', 'join_t', 'meet_t', 'join_f', 'meet_f')]
CASES += [[x, FRAME, SEED] for x in ('retain-after-inject', 'inject-after-retain')]
CASES += [['program', 'Y', 'K', 'K', 'data:' + SEED, '@', '@', '@'],
          ['orbit', SEED, str(SOURCE), 'negation'], ['word', WORD, '0', str(SOURCE)]]
records = []
for repo, exe in [('Vox', 'vox'), ('G-mOMonadOS', 'g-momonados')]:
    binary = ROOT / repo / 'target/release' / exe
    for index, args in enumerate(CASES):
        label = f'{exe}_{index:02d}_{args[0]}'
        result = subprocess.run([str(binary), 'reentry', *args], capture_output=True,
                                text=True, timeout=90)
        (HERE / (label + '.stdout')).write_text(result.stdout)
        (HERE / (label + '.stderr')).write_text(result.stderr)
        rows = result.stdout.splitlines()
        table = next((ast.literal_eval(x[6:]) for x in rows if x.startswith('table=')), None)
        candidates = sorted({int(x) for x in re.findall(r'(?:least|greatest|value)=(\d+)', result.stdout) if int(x) > 1})
        checks = [{'candidate': x, 'remainder': str(SOURCE % x),
                   'factor': SOURCE % x == 0 and x < SOURCE} for x in candidates]
        record = {'tool': exe, 'operation': args[0], 'exit': result.returncode,
                  'table': table, 'candidate_checks': checks,
                  'report': result.stdout}
        records.append(record)
        (HERE / 'sweep.json').write_text(json.dumps(records, indent=2))
        print(json.dumps({k: record[k] for k in ('tool', 'operation', 'exit', 'candidate_checks')}), flush=True)
        assert result.returncode == 0, result.stderr

# A different full numeral tests whether the readout discriminates source values.
control = subprocess.run([str(ROOT / 'Vox/target/release/vox'), 'numeral', str(SOURCE + 2)],
                         capture_output=True, text=True, timeout=90)
assert control.returncode == 0, control.stderr
control_word = control.stdout.strip()
(HERE / 'control_plus_two.imasm').write_text(control_word + '\n')
result = subprocess.run([str(ROOT / 'Vox/target/release/vox'), 'reentry', 'word', control_word],
                        capture_output=True, text=True, timeout=90)
(HERE / 'control_plus_two.stdout').write_text(result.stdout)
assert result.returncode == 0, result.stderr
print('control plus two: ' + result.stdout.replace('\n', ' | '), flush=True)
