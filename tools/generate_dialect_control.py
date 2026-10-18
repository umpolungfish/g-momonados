#!/usr/bin/env python3
"""Generate a fresh balanced source, keeping all reference factors separate."""
import argparse
import hashlib
import json
from pathlib import Path
import secrets
import subprocess
from sympy import isprime
from verify_dialect_factor import numeral_word

ROOT = Path(__file__).resolve().parents[1]
BRIDGE = r'''
use g_momonados::word_tape::WordTape;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3);
    let left = WordTape::from_canonical_word(std::fs::read_to_string(&args[1]).unwrap().trim()).unwrap();
    let right = WordTape::from_canonical_word(std::fs::read_to_string(&args[2]).unwrap().trim()).unwrap();
    let source = left.mul(&right);
    let (quotient, remainder) = source.divmod(&left).unwrap();
    assert!(remainder.is_zero());
    assert_eq!(quotient, right);
    println!("{{\"source_word\":\"{}\",\"source_bits\":{},\"word_product_and_quotient\":true}}",
        source.as_word(), source.bit_len());
}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('reference', type=Path)
    parser.add_argument('--bits', type=int, default=208)
    args = parser.parse_args()
    assert args.bits >= 200 and args.bits % 2 == 0
    source, reference = args.source.resolve(), args.reference.resolve()
    assert not source.exists() and not reference.exists(), 'preserve existing sources and references'
    reference.mkdir(parents=True)
    source.parent.mkdir(parents=True, exist_ok=True)
    bridge = reference / 'word_product.rs'
    bridge.write_text(BRIDGE)
    build = ROOT / 'target/x86_64-unknown-linux-gnu/release'
    library = max((build / 'deps').glob('libg_momonados-*.rlib'), key=lambda p: p.stat().st_mtime)
    executable = reference / 'word_product'
    command = ['rustc', '--edition=2021', '-Dwarnings', '-Cpanic=abort', '--target', 'x86_64-unknown-linux-gnu',
               str(bridge), '--extern', f'g_momonados={library}', '-L',
               f'dependency={build / "deps"}', '-o', str(executable)]
    with (reference / 'build.log').open('wb') as log:
        subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
    half = args.bits // 2

    def prime():
        while True:
            candidate = secrets.randbits(half) | (1 << (half - 1)) | 1
            if isprime(candidate):
                return candidate

    while True:
        left, right = prime(), prime()
        if left == right:
            continue
        for name, value in [('left', left), ('right', right)]:
            (reference / (name + '.imasm')).write_text(numeral_word(value) + '\n')
        raw = subprocess.check_output([str(executable), str(reference / 'left.imasm'), str(reference / 'right.imasm')])
        product = json.loads(raw)
        if product['source_bits'] == args.bits:
            break
    source.write_text(product['source_word'] + '\n')
    (reference / 'word_product.json').write_bytes(raw)
    record = {
        'format': 'dialect-fresh-control-v1', 'source_bits': args.bits,
        'reference_prime_bits': [half, half], 'distinct_reference_primes': True,
        'generator': 'independent secrets random odd candidates filtered by reference SymPy isprime',
        'product_operation': 'existing WordTape::mul on canonical IMASM numeral words',
        'reference_verification': 'WordTape::divmod gives the other arm with zero remainder',
        'production_boundary': 'source numeral file only; no reference files passed to the baker',
        'source': str(source), 'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
        'reference_bindings': {str(p): hashlib.sha256(p.read_bytes()).hexdigest()
                               for p in [bridge, executable, library, reference / 'left.imasm', reference / 'right.imasm']}}
    (reference / 'generation.json').write_text(json.dumps(record, indent=2) + '\n')
    print(json.dumps({k: v for k, v in record.items() if k != 'reference_bindings'}, indent=2))


if __name__ == '__main__':
    main()
