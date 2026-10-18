#!/usr/bin/env python3
"""Bake exact sibling-dialect difference/fuse checks from a verified held return."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time

parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('project',type=Path)
parser.add_argument('returned',type=Path)
parser.add_argument('verification',type=Path)
parser.add_argument('output',type=Path)
parser.add_argument('--coordinate',choices=['candidate','anchor-wedge'],default='candidate')
args=parser.parse_args()
project=args.project.resolve();output=args.output.resolve()
if output.exists() or Path(str(output)+'.source').exists():raise SystemExit('preserve existing probe output')
started=time.monotonic();digest=hashlib.sha256()
with args.returned.open('rb') as stream:
    for chunk in iter(lambda:stream.read(1024*1024),b''):digest.update(chunk)
verified=json.loads(args.verification.read_text())
if digest.hexdigest()!=verified['receipt_sha256']:raise SystemExit('held return is not bound to the supplied readback')
data=json.loads(args.returned.read_text());tables=json.loads((project/'dialect_tables.json').read_text())
if data['source_word']!=tables['source_word'] or data['closed']:raise SystemExit('probe requires the same open source-bound return')
width=tables['source_bits']+10
if args.coordinate=='anchor-wedge':
    if not all(member.get('product_frame_state',{}).get('initialized') for member in data['members']):
        raise SystemExit('anchor wedge requires the retained product frame')
    states=[member['product_frame_state']['anchor_left'] for member in data['members']]
    partners=[member['product_frame_state']['anchor_right'] for member in data['members']]
else:
    states=[member['state'][:width] for member in data['members']]
    partners=[[8]*width]
destination=Path(str(output)+'.source');destination.mkdir(parents=True)
arrays=',\n'.join('{'+','.join(map(str,state))+'}' for state in states)
program='#define main retained_dialect_main\n#include '+json.dumps(str(project/'membrane.cpp'))+'\n#undef main\n'
program+=f'static const arm HELD_CANDIDATES[][W]={{{arrays}}};\n'
program+='static const arm HELD_PARTNERS[][W]={'+',\n'.join('{'+','.join(map(str,state))+'}' for state in partners)+'};\n'
program+='constexpr bool HELD_ANCHOR_WEDGE='+str(args.coordinate=='anchor-wedge').lower()+';\n'
program+='static const char HELD_COORDINATE[]='+json.dumps(args.coordinate)+';\n'
program+=r'''
int main(int argc,char**){
 if(argc!=1)return 1;
 const auto trigger=std::chrono::steady_clock::now();Word n;load_word(SOURCE_ARMS,n);
 unsigned long long pairs=0,nonzero=0;bool closed=false;unsigned left=0,right=0;
 ResidentState result{};arm paired[2*W]{};
 for(unsigned i=0;i<sizeof(HELD_CANDIDATES)/sizeof(HELD_CANDIDATES[0])&&!closed;++i){
  Word a;load_word(HELD_CANDIDATES[i],a);
  for(unsigned j=0;j<i;++j){Word b,difference;load_word(HELD_CANDIDATES[j],b);++pairs;
   if(HELD_ANCHOR_WEDGE){
    Word qa,qb,left,right;load_word(HELD_PARTNERS[i],qa);load_word(HELD_PARTNERS[j],qb);
    regular_mulmod(a,qb,n,left);regular_mulmod(b,qa,n,right);submod(left,right,n,difference);
   }else if(compare(a,b)>=0)subtract(a,b,difference);else subtract(b,a,difference);
   if(zero(difference)){continue;}
   ++nonzero;
   if(fuse_source_arm(difference,n,result,paired)){closed=true;left=i;right=j;break;}
  }
 }
 std::cout<<"{\"source_word\":\""<<dialect_numeral(SOURCE_ARMS)<<"\",\"coordinate\":\""<<HELD_COORDINATE<<"\",\"pairs\":"<<pairs
  <<",\"nonzero_differences\":"<<nonzero<<",\"closed\":"<<(closed?"true":"false")
  <<",\"left_member\":"<<left<<",\"right_member\":"<<right
  <<",\"trigger_to_readout_seconds\":"<<std::chrono::duration<double>(std::chrono::steady_clock::now()-trigger).count()
  <<",\"divisor_word\":\""<<dialect_numeral(result.divisor.v)<<"\",\"quotient_word\":\""<<dialect_numeral(result.quotient.v)<<"\"}\n";
 return closed?0:2;
}
'''
(destination/'probe.cpp').write_text(program)
payload_object=project/'retained_payload.o'
link_inputs=[str(payload_object)] if payload_object.exists() else []
if link_inputs:
    compiled=json.loads((project/'compilation.json').read_text())
    assert compiled['bindings'][str(payload_object)]==hashlib.sha256(payload_object.read_bytes()).hexdigest(), 'retained payload binding changed'
with (destination/'build.log').open('wb') as log:
    subprocess.run(['g++','-std=c++17','-O2','-static','-Wall','-Wextra','-Werror','-Wno-unknown-pragmas',str(destination/'probe.cpp'),*link_inputs,'-o',str(output)],stdout=log,stderr=subprocess.STDOUT,check=True)
compilation=time.monotonic()-started
with (destination/'linkage.txt').open('wb') as log:subprocess.run(['readelf','-l','-d',str(output)],stdout=log,check=True)
text=(destination/'linkage.txt').read_text()
assert 'INTERP' not in text and '(NEEDED)' not in text
receipt={'parent_return_sha256':digest.hexdigest(),'parent_executable_project':str(project),
         'source_bits':tables['source_bits'],'members':len(states),'compilation_seconds':compilation,
         'coordinate':args.coordinate,
         'binary_sha256':hashlib.sha256(output.read_bytes()).hexdigest(),'static':True,
         'retained_payload_bindings':{path:hashlib.sha256(Path(path).read_bytes()).hexdigest() for path in link_inputs},
         'boundary':'source-only generated candidate return; no reference factors'}
(destination/'compilation.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt,indent=2))
