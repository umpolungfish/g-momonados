#!/usr/bin/env python3
"""Execute canonical primitive programs on the native full register carrier.

This is a bridge diagnostic, not a factor producer. Read static primitive
definitions only; no ob3ect generator or model is invoked. Typed opcodes and
explicit fork/fuse pairs survive transport without a glyph-only reparse.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
CORE = ROOT / 'G-mOMonadOS/vendor/imasm_core/src/lib.rs'
HARNESS = r'''
extern crate imasm_core;
use imasm_core::{classic::Token, check, flow, imasm16_3::Reg16_3};
fn mask(v: Reg16_3) -> u8 {
    v.big_t as u8 | (v.big_f as u8)<<1 | (v.small_t as u8)<<2 | (v.small_f as u8)<<3
}
fn main() {
    let path = std::env::args().nth(1).expect("type table path");
    for line in std::fs::read_to_string(path).unwrap().lines() {
        let parts: Vec<_> = line.split('\t').collect();
        let ops: Vec<_> = parts[1].split(',').map(|n| Token::parse(n).expect("typed opcode")).collect();
        let pairs: Vec<_> = if parts[2].is_empty() { vec![] } else {
            parts[2].split(',').map(|p| { let (a,b)=p.split_once(':').unwrap();
                (a.parse::<usize>().unwrap(),b.parse::<usize>().unwrap()) }).collect()
        };
        let graph = check::from_sequence(&ops, &pairs);
        for seed in 0..16u8 {
            let value = Reg16_3 { big_t:seed&1!=0, big_f:seed&2!=0,
                small_t:seed&4!=0, small_f:seed&8!=0 };
            let (nodes,edges)=flow::flow_values(&graph,value);
            let mut terminal=Reg16_3::default();
            for (i,op) in graph.nodes.iter().enumerate() {
                if *op==Token::Tanch { terminal=terminal.union(nodes[i]); }
            }
            println!("{}\t{}\t{}\t{:?}\t{:?}",parts[0],seed,mask(terminal),
                nodes.into_iter().map(mask).collect::<Vec<_>>(),
                edges.into_iter().map(mask).collect::<Vec<_>>());
        }
    }
}
'''


def run(cmd):
    subprocess.run([str(x) for x in cmd], check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output_directory', type=Path)
    parser.add_argument('--type-root', type=Path, default=ROOT / 'ob3ect/digital')
    args = parser.parse_args()
    out = args.output_directory
    out.mkdir(parents=True, exist_ok=False)
    definitions = []
    table = []
    paths = list(args.type_root.glob('the_primitive_type_called_*_ob3ect.json'))
    paths += list(args.type_root.glob('the_primitive_type_called_*/the_primitive_type_called_*_ob3ect.json'))
    for path in sorted(paths):
        raw = path.read_bytes()
        data = json.loads(raw)
        name = path.name.removeprefix('the_primitive_type_called_').removesuffix('_ob3ect.json')
        steps = data['phases']['phase_4']['steps']
        ops = [s['opcode'] for s in steps]
        pairs = data.get('topology_report', {}).get('pair_list', [])
        if not ops or any(not (0 <= a < b < len(ops)) for a, b in pairs):
            raise ValueError(f'invalid source program/pairs: {path}')
        definitions.append({'name': name, 'source': str(path),
                            'sha256': hashlib.sha256(raw).hexdigest(),
                            'typed_opcodes': ops, 'pairs': pairs})
        table.append('\t'.join([name, ','.join(ops),
                                ','.join(f'{a}:{b}' for a, b in pairs)]))
    if not definitions:
        raise ValueError('no canonical primitive programs')
    (out / 'programs.tsv').write_text('\n'.join(table) + '\n')
    (out / 'execute.rs').write_text(HARNESS)
    run(['rustc', '--edition=2021', '--crate-name', 'imasm_core', '--crate-type', 'rlib',
         '-O', CORE, '-o', out / 'libimasm_core.rlib'])
    run(['rustc', '--edition=2021', '-O', out / 'execute.rs', '--extern',
         f'imasm_core={out / "libimasm_core.rlib"}', '-o', out / 'execute'])
    result = subprocess.run([str(out / 'execute'), str(out / 'programs.tsv')],
                            check=True, capture_output=True, text=True)
    records = {}
    for line in result.stdout.splitlines():
        name, seed, terminal, nodes, edges = line.split('\t')
        records.setdefault(name, []).append({'seed': int(seed), 'terminal': int(terminal),
                                             'node_inputs': json.loads(nodes),
                                             'edge_values': json.loads(edges)})
    for definition in definitions:
        rows = records[definition['name']]
        if [r['seed'] for r in rows] != list(range(16)):
            raise ValueError('incomplete full-carrier execution')
        definition['executions'] = rows
        definition['terminal_image_size'] = len({r['terminal'] for r in rows})
        definition['terminal_is_identity'] = all(r['seed'] == r['terminal'] for r in rows)
    manifest = {'instrument_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                'core_sources': {str(p): hashlib.sha256(p.read_bytes()).hexdigest()
                                 for p in sorted(CORE.parent.glob('*.rs'))},
                'readout': 'union of inputs to all TANCH nodes; complete node and edge state retained',
                'scope': 'canonical type programs on all SIXTEEN_3 seeds; no arithmetic or factor claim',
                'programs': definitions}
    (out / 'execution.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps({'programs': len(definitions), 'carrier_executions': len(result.stdout.splitlines()),
                      'terminal_injective': sum(d['terminal_image_size'] == 16 for d in definitions),
                      'terminal_identity': sum(d['terminal_is_identity'] for d in definitions)}))


if __name__ == '__main__':
    main()
