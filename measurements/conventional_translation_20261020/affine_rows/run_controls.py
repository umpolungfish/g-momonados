from pathlib import Path
import subprocess,json,time,hashlib
BASE=Path(__file__).resolve().parent
PROJECT=BASE.parents[2]
TARGET=PROJECT/'target/conventional_translation_20261020/x86_64-unknown-linux-gnu/release'
source=BASE/'fresh_balanced_200.txt'
commands=[]
for name,file in [('operator','operator_probe.rs'),('protocol','../protocol_probe.rs'),('prefix','prefix_probe.rs')]:
 command=['rustc','--edition=2021','-C','opt-level=2','-C','panic=abort','--target','x86_64-unknown-linux-gnu','-L','dependency='+str(TARGET/'deps'),'--extern','g_momonados='+str(TARGET/'libg_momonados.rlib')]
 if name=='operator':command+=['--extern','num_bigint='+str(TARGET/'deps/libnum_bigint-54692d642badf259.rlib')]
 executable=BASE/(name+'_probe')
 command += [str(BASE/file),'-o',str(executable)]
 subprocess.run(command,check=True)
 start=time.monotonic()
 with (BASE/(name+'_final.stdout')).open('w') as out,(BASE/(name+'_final.stderr')).open('w') as err:
  result=subprocess.run(['timeout','-s','INT','-k','5s','300s',str(executable),str(source)],stdout=out,stderr=err)
 receipt={'instrument':name,'returncode':result.returncode,'elapsed_seconds':time.monotonic()-start,'source_bits':int(source.read_text()).bit_length(),'N_only_file_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'executable_sha256':hashlib.sha256(executable.read_bytes()).hexdigest()}
 commands.append(receipt)
 (BASE/'controls.json').write_text(json.dumps(commands,indent=2)+'\n')
 print(json.dumps(receipt),flush=True)
 if result.returncode:raise SystemExit(result.returncode)
