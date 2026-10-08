import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent
BIN = Path('/home/mrnob0dy666/imsgct/G-mOMonadOS/target/release/g-momonados')
SIZES = (256, 512, 1024, 1048)
CAP_SECONDS = 30
results = []

for bits in SIZES:
    size_dir = ROOT / str(bits)
    word = (size_dir / 'source.imasm').read_text().strip()
    if not word.startswith('⊢') or any(ch.isascii() and ch.isdigit() for ch in word):
        raise SystemExit(f'{bits}: source is not a numeral word')
    log_path = size_dir / 'g-factor-membrane-word.log'
    started = time.monotonic()
    try:
        run = subprocess.run([str(BIN), 'factor_membrane', 'factor', word],
                             cwd=BIN.parent.parent, capture_output=True,
                             text=True, timeout=CAP_SECONDS)
        elapsed = time.monotonic() - started
        status = 'exit'
        returncode = run.returncode
        output = run.stdout + run.stderr
    except subprocess.TimeoutExpired as exc:
        elapsed = time.monotonic() - started
        status = 'timeout'
        returncode = None
        out = exc.stdout or b''
        err = exc.stderr or b''
        output = out.decode(errors='replace') if isinstance(out, bytes) else out
        output += err.decode(errors='replace') if isinstance(err, bytes) else err
    log_path.write_text(output)
    row = {'bits': bits, 'name': 'G factor_membrane factor',
           'input_representation': 'canonical IMASM numeral word',
           'elapsed_seconds': round(elapsed, 6), 'timeout_seconds': CAP_SECONDS,
           'status': status, 'returncode': returncode,
           'output_file': f'{bits}/g-factor-membrane-word.log'}
    results.append(row)
    (ROOT / 'g_factor_membrane_word_sweep.json').write_text(json.dumps(results, indent=2) + '\n')
    print(json.dumps(row), flush=True)
