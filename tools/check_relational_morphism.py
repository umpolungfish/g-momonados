#!/usr/bin/env python3
"""Exercise native morphisms on the actual joint cells of a prepared source."""
from pathlib import Path
import argparse
import hashlib
import json
import subprocess

ROOT = Path(__file__).resolve().parents[2]
HARNESS = r'''
extern crate imasm_core;
#[path="MODULE"] mod relational_morphism;
use imasm_core::{check,classic::Token};
use relational_morphism::JointRelation;
fn main(){
 let path=std::path::PathBuf::from(std::env::args().nth(1).unwrap());
 let source=std::fs::read_to_string(path.join("source.imasm")).unwrap();
 let packed=std::fs::read(path.join("relation.cells")).unwrap();
 let rows=std::fs::read(path.join("cell.rows")).unwrap();
 let domains=std::fs::read(path.join("register.cells")).unwrap();
 assert_eq!(rows.len(),40);assert_eq!(packed.len()%20,0);
 let reverse=check::from_sequence(&[Token::Vinit,Token::Arev,Token::Tanch],&[]);
 let truth=check::from_sequence(&[Token::Vinit,Token::Evalt,Token::Tanch],&[]);
 let mut cells=0;let mut changed=0;let mut order_sensitive=0;let mut live_rows=0;
 for cell in packed.chunks_exact(20){
  let ports:Vec<_>=cell.chunks_exact(4).map(|v|u32::from_le_bytes(v.try_into().unwrap()) as usize).collect();
  let active:Vec<Vec<u8>>=rows.chunks_exact(5).filter(|row|
   row.iter().zip(&ports).all(|(&v,&p)|domains[p]&v!=0)).map(|r|r.to_vec()).collect();
  let relation=JointRelation::new(source.clone(),ports,active).unwrap();
  let first=relation.through("AREV",&reverse).unwrap().relation;
  let second=first.through("AREV",&reverse).unwrap().relation;
  assert_eq!(second.rows,relation.rows);assert_eq!(second.origins,relation.origins);
  assert_eq!(second.source_word,source);assert_eq!(second.ports,relation.ports);
  changed+=usize::from(first.rows!=relation.rows);
  let sequential=first.through("EVALT",&truth).unwrap().relation;
  let direct=relation.through("EVALT",&truth).unwrap().relation;
  order_sensitive+=usize::from(sequential.rows!=direct.rows);
  assert_eq!(sequential.origins,relation.origins);
  live_rows+=relation.rows.len();cells+=1;
 }
 println!("{{\"cells\":{},\"joint_rows\":{},\"changed_cells\":{},\"sequential_effect_cells\":{},\"involution_recovery\":true}}",cells,live_rows,changed,order_sensitive);
}
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('prepared_directory', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    module = ROOT / 'G-mOMonadOS/tools/relational_morphism.rs'
    core = ROOT / 'G-mOMonadOS/vendor/imasm_core/src/lib.rs'
    harness = args.output / 'check.rs'
    harness.write_text(HARNESS.replace('MODULE', str(module)))
    library = args.output / 'libimasm_core.rlib'
    commands = [
        ['rustc', '--edition=2021', '--crate-name', 'imasm_core', '--crate-type', 'rlib', '-O', str(core), '-o', str(library)],
        ['rustc', '--edition=2021', '-O', str(harness), '--extern', f'imasm_core={library}', '-o', str(args.output / 'check')],
    ]
    with (args.output / 'build.log').open('w') as log:
        for command in commands:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
    result = subprocess.run([str(args.output / 'check'), str(args.prepared_directory)], capture_output=True, text=True, check=True)
    report = json.loads(result.stdout)
    paths = [module, core, *(args.prepared_directory / name for name in ['source.imasm', 'relation.cells', 'cell.rows', 'register.cells'])]
    report['bindings'] = {str(p.resolve()): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
    report['scope'] = 'native graph lift on source-bound joint arithmetic cells; dialect dispatch remains to be connected'
    (args.output / 'receipt.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))


if __name__ == '__main__':
    main()
