#!/usr/bin/env python3
"""Locate a bounded Vox execution internally, emitting observations only at return."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT=Path(__file__).resolve().parents[2]

DRIVER=r'''
use vox::imasm_vm::{self, Host, Machine};
use std::{cell::RefCell,rc::Rc,time::{Duration,Instant}};
include!("std_host.rs");
struct Boundary;
struct Observed {inner:StdHost, started:Instant, seconds:u64, samples:Rc<RefCell<Vec<(u64,u64,f64)>>>}
impl Host for Observed {
 fn open(&mut self,p:&str,f:i32,m:i32)->i32{self.inner.open(p,f,m)}
 fn read(&mut self,f:i32,b:&mut[u8])->i64{self.inner.read(f,b)}
 fn write(&mut self,f:i32,b:&[u8])->i64{self.inner.write(f,b)}
 fn close(&mut self,f:i32)->i32{self.inner.close(f)}
 fn pipe(&mut self,f:i32)->Result<(i32,i32),i32>{self.inner.pipe(f)}
 fn clock_gettime(&mut self,c:i32)->Result<(i64,i64),i32>{self.inner.clock_gettime(c)}
 fn file_stat(&mut self,f:i32)->Result<imasm_vm::FileStat,i32>{self.inner.file_stat(f)}
 fn pread(&mut self,f:i32,b:&mut[u8],o:u64)->i64{self.inner.pread(f,b,o)}
 fn ioctl(&mut self,f:i32,r:u64,a:&mut[u8],b:&mut[(usize,Vec<u8>)])->i64{self.inner.ioctl(f,r,a,b)}
 fn dup2(&mut self,a:i32,b:i32)->i32{self.inner.dup2(a,b)}
 fn chdir(&mut self,p:&str)->i32{self.inner.chdir(p)}
 fn exec(&mut self,p:&str,a:&[String],e:&[String])->Result<i32,i32>{self.inner.exec(p,a,e)}
 fn wait(&mut self,p:i32)->Result<(i32,i32),i32>{self.inner.wait(p)}
 fn begin_fork(&mut self){self.inner.begin_fork()}
 fn end_fork(&mut self){self.inner.end_fork()}
 fn execution_progress(&mut self,steps:u64,pc:u64){
   self.samples.borrow_mut().push((steps,pc,self.started.elapsed().as_secs_f64()));
   if self.started.elapsed()>=Duration::from_secs(self.seconds){std::panic::panic_any(Boundary);}
 }
}
fn main(){
 let args:Vec<String>=std::env::args().collect();let started=Instant::now();
 let module=std::fs::read_to_string(&args[1]).unwrap();let read_seconds=started.elapsed().as_secs_f64();
 vox::imasm_module::validate_standalone(&module).unwrap();
 let validated_seconds=started.elapsed().as_secs_f64();let mut machine=Machine::new(&module);
 let loaded_seconds=started.elapsed().as_secs_f64();
 let samples=Rc::new(RefCell::new(Vec::new()));
 machine.set_host(Box::new(Observed{inner:StdHost::new(),started:Instant::now(),seconds:args[2].parse().unwrap(),samples:Rc::clone(&samples)}));
 std::panic::set_hook(Box::new(|_|{}));
 let evidence=std::path::Path::new(&args[3]).parent().unwrap().join("evidence");
 std::fs::create_dir_all(&evidence).unwrap();
 let guest_env=vec![format!("RESIDENT_EVIDENCE_DIR={}",evidence.display())];
 let outcome=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||machine.run_process(&args[1..2],&guest_env,u64::MAX)));
 let status=match outcome {Ok(Ok(()))=>"returned".to_string(),Ok(Err(imasm_vm::Stop::SysExit(c)))=>format!("exit {c}"),
   Ok(Err(imasm_vm::Stop::Halt(e)))=>format!("halt {e}"),Err(e)=>if e.is::<Boundary>(){"diagnostic wall boundary".to_string()}else{"host panic".to_string()}};
 let mut symbols:Vec<_>=machine.symbols.iter().map(|(name,&address)|(address,name.clone())).collect();symbols.sort();
 let observed:Vec<_>=samples.borrow().iter().map(|&(steps,pc,seconds)|{
   let at=symbols.partition_point(|(address,_)|*address<=pc);
   serde_json::json!({"steps":steps,"pc":pc,"seconds":seconds,"symbol":if at>0{symbols[at-1].1.as_str()}else{"unknown"}})
 }).collect();
 let registers:std::collections::BTreeMap<_,_>=["rax","rbx","rcx","rdx","rsi","rdi","rbp","rsp","r12","r13","r14","r15"].iter()
   .map(|&r|(r,format!("{:x}",machine.reg(r)))).collect();
 let report=serde_json::json!({"status":status,"read_seconds":read_seconds,"validated_seconds":validated_seconds,
   "loaded_seconds":loaded_seconds,"elapsed_seconds":started.elapsed().as_secs_f64(),"steps":machine.steps,"samples":observed,"registers":registers});
 std::fs::write(&args[3],serde_json::to_vec_pretty(&report).unwrap()).unwrap();
}
'''


def main(directory,output,seconds):
    output=output.resolve()
    if (output/'receipt.json').exists():raise ValueError('completed Vox observation already exists')
    (output/'src').mkdir(parents=True,exist_ok=True)
    cli=ROOT/'Vox/src/main.rs';text=cli.read_text();start=text.index('struct StdHost {')
    stop=text.index('#[cfg(test)]',start)
    (output/'src/std_host.rs').write_text(text[start:stop])
    (output/'src/main.rs').write_text(DRIVER)
    (output/'Cargo.toml').write_text('[package]\nname="resident-vox-diagnostic"\nversion="0.1.0"\nedition="2021"\n[dependencies]\nserde_json="1"\nvox={path='+json.dumps(str(ROOT/'Vox'))+'}\n[profile.release]\nopt-level=3\n')
    env=dict(os.environ,CARGO_TARGET_DIR=str(output.parent/'vox_diagnostic_cargo'))
    env.pop('VOX_PROGRESS',None)
    with (output/'build.log').open('wb') as log:
        subprocess.run(['cargo','build','--offline','--release','--target','x86_64-unknown-linux-gnu','--manifest-path',str(output/'Cargo.toml')],
                       env=env,stdout=log,stderr=subprocess.STDOUT,check=True)
    binary=output.parent/'vox_diagnostic_cargo/x86_64-unknown-linux-gnu/release/resident-vox-diagnostic'
    with (output/'guest.stdout').open('wb') as out,(output/'guest.stderr').open('wb') as err:
        subprocess.run([str(binary),str(directory/'membrane.imasm'),str(seconds),str(output/'observation.json')],
                       env=env,stdout=out,stderr=err,check=True)
    paths=[cli,ROOT/'Vox/src/imasm_vm.rs',directory/'membrane.imasm',Path(__file__),binary]
    receipt={'bindings':{str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths},
             'scope':'bounded internal guest-address observations; emitted after diagnostic return',
             'observation':json.loads((output/'observation.json').read_text())}
    (output/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    observation=receipt['observation'];print(json.dumps({k:observation[k] for k in ['status','loaded_seconds','elapsed_seconds','steps']}))
    print(json.dumps(observation['samples'][-3:]))


if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('directory',type=Path);p.add_argument('output',type=Path)
    p.add_argument('--seconds',type=int,default=60);a=p.parse_args();main(a.directory,a.output,a.seconds)
