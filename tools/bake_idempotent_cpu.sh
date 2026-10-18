#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/../../Vox/scripts/static_membrane.sh"
[[ $# == 2 ]] || { printf 'usage: %s PREPARED_MEMBRANE_SOURCE OUTPUT\nDefault compilation prepares through verified factor closure without a time bound. CPU_PREPARE_TO_CLOSURE=0 emits an open diagnostic image.\n' "$0" >&2; exit 2; }
cpu_root=$(cd "$(dirname "$0")/.." && pwd)
cpu_prepared=$(realpath "$1")
cpu_output=$(realpath -m "$2")
cpu_project="$cpu_output.cpu-source"
[[ ! -e $cpu_output && ! -e $cpu_project ]] || { printf 'output already exists\n' >&2; exit 2; }
mkdir -p "$cpu_project"
cpu_started=$SECONDS
cpu_preparation_flags=()
case ${CPU_PREPARE_TO_CLOSURE:-1} in
  1) cpu_preparation_flags=(-DPREPARE_TO_CLOSURE) ;;
  0) ;;
  *) printf 'CPU_PREPARE_TO_CLOSURE must be zero or one\n' >&2; exit 2 ;;
esac
node - "$cpu_prepared" "$cpu_project" "$cpu_root" <<'JS'
const fs=require('fs'),path=require('path');
const [prepared,output,root]=process.argv.slice(2);
const cuda=fs.readFileSync(path.join(prepared,'membrane.cu'),'utf8');
const data=JSON.parse(fs.readFileSync(path.join(prepared,'preparation.json'),'utf8'));
const width=Number(/^#define W (\d+)$/m.exec(cuda)[1]);
if(width!==data.source_bits+10 || data.state.length!==32*14*width || data.continuation.length!==32*6146 || data.receipt.length!==330)
  throw Error('prepared bank dimensions do not match the source');
const header=cuda.slice(0,cuda.indexOf('// Every operand'));
const gcdStart=cuda.indexOf('// n is odd, so binary gcd');
const bodyStart=cuda.indexOf('// Uses the existing warp-distributed');
if(gcdStart<0||bodyStart<gcdStart)throw Error('prepared CUDA source lacks required operation boundaries');
const arithmetic=cuda.slice(gcdStart,bodyStart);
let body=fs.readFileSync(path.join(root,'src/idempotent_gpu.cu'),'utf8');
body=body.replace(/__device__ void load_word\([\s\S]*?\n}/,'__device__ void load_word(const arm *p,Word &w){std::copy(p,p+W,w.v);}')
  .replace(/__device__ void save_word\([\s\S]*?\n}/,'__device__ void save_word(const Word &w,arm *p){std::copy(w.v,w.v+W,p);}')
  .replaceAll('int i=32*k+lane()','int i=k').replaceAll('i+=32','++i')
  .replaceAll('if(lane()==i%32)','').replaceAll('if(lane()==bit%32)','')
  .replaceAll('.v[i/32]','.v[i]').replaceAll('.v[bit/32]','.v[bit]');
const widthLine=`#define W ${width}\n`;
const banks=`\nstatic const arm PREPARED_STATE[]={${data.state}};\nstatic const int PREPARED_CONTINUATION[]={${data.continuation}};\nstatic const unsigned long long PREPARED_RECEIPT[]={${data.receipt}};\n`;
const program=widthLine+fs.readFileSync(path.join(root,'src/idempotent_cpu_word.hpp'),'utf8')+'\n'+header+arithmetic+body+'\n// BEGIN PREPARED BANKS\n'+banks+'// END PREPARED BANKS\n'+fs.readFileSync(path.join(root,'src/idempotent_cpu_runner.hpp'),'utf8');
fs.writeFileSync(path.join(output,'membrane.cpp'),program);
fs.writeFileSync(path.join(output,'preparation_binding.txt'),prepared+'\n');
JS
g++ -std=c++17 -O2 -static -Wall -Wextra -Werror -Wno-unknown-pragmas "${cpu_preparation_flags[@]}" -DPREPARATION_EMIT "$cpu_project/membrane.cpp" -o "$cpu_project/prepare_cpu"
"$cpu_project/prepare_cpu" > "$cpu_project/preparation.json" 2> "$cpu_project/preparation_emit.log"
node - "$cpu_project" <<'JS'
const fs=require('fs'),path=require('path');const project=process.argv[2];
const d=JSON.parse(fs.readFileSync(path.join(project,'preparation.json'),'utf8'));
if(d.state.length!==32*14*(d.source_bits+10)||d.continuation.length!==32*6146||d.receipt.length!==330)
  throw Error('CPU preparation emitted invalid bank dimensions');
const p=path.join(project,'membrane.cpp');let text=fs.readFileSync(p,'utf8');
const banks=`\nstatic const arm PREPARED_STATE[]={${d.state}};\nstatic const int PREPARED_CONTINUATION[]={${d.continuation}};\nstatic const unsigned long long PREPARED_RECEIPT[]={${d.receipt}};\n`;
text=text.replace(/\/\/ BEGIN PREPARED BANKS\n[\s\S]*?\/\/ END PREPARED BANKS/,`// BEGIN PREPARED BANKS\n${banks}// END PREPARED BANKS`);
fs.writeFileSync(p,text);
JS
g++ -std=c++17 -O2 -static -Wall -Wextra -Werror -Wno-unknown-pragmas "${cpu_preparation_flags[@]}" -DPREPARATION_CHECK "$cpu_project/membrane.cpp" -o "$cpu_project/verify_preparation"
"$cpu_project/verify_preparation" > "$cpu_project/preparation_check.log" 2>&1
g++ -std=c++17 -O2 -static -Wall -Wextra -Werror -Wno-unknown-pragmas "$cpu_project/membrane.cpp" -o "$cpu_output"
require_static_membrane "$cpu_output"
readelf -l -d "$cpu_output" > "$cpu_output.linkage.txt"
if rg -q 'INTERP|\(NEEDED\)' "$cpu_output.linkage.txt"; then printf 'CPU membrane is not static\n' >&2; exit 1; fi
sha256sum "$cpu_output" "$cpu_project/membrane.cpp" "$cpu_project/preparation.json" "$cpu_prepared/preparation.json" "$cpu_prepared/membrane.cu" "$cpu_root/src/idempotent_gpu.cu" "$cpu_root/src/idempotent_cpu_word.hpp" "$cpu_root/src/idempotent_cpu_runner.hpp" > "$cpu_output.bindings.sha256"
cpu_vox=${VOX_BIN:-"$cpu_root/../Vox/target/x86_64-unknown-linux-gnu/release/vox"}
if [[ ! -v VOX_BIN ]]; then
  (cd "$cpu_root/../Vox" && cargo build --target-dir target --offline --release --bin vox --target x86_64-unknown-linux-gnu)
fi
"$cpu_vox" imasm "$cpu_output" > "$cpu_output.imasm"
sha256sum "$cpu_output.imasm" "$cpu_vox" >> "$cpu_output.bindings.sha256"
printf 'baked CPU membrane = %s\ncompilation_seconds = %s\n' "$cpu_output" "$((SECONDS-cpu_started))"
cat "$cpu_project/preparation_check.log"
