import os,pty,select,subprocess,time,termios,json,sys
from pathlib import Path
binary=str(Path(os.environ.get('BINARY','target/release/g-momonados')).resolve())
n='1443886668465785239555226711752594674554632143872728785072260425128493389058547185473075869'
word=subprocess.check_output([binary,'native_numeral','encode',n]).decode().strip()
command=' '*int(os.environ.get('PADDING','0'))+'native_numeral deinterlace '+word
master,slave=pty.openpty();before=termios.tcgetattr(slave)
p=subprocess.Popen([binary],stdin=slave,stdout=slave,stderr=slave,start_new_session=True)
data=bytearray()
def until(marker,seconds=30):
 end=time.monotonic()+seconds
 while time.monotonic()<end:
  if select.select([master],[],[],.1)[0]:
   chunk=os.read(master,65536);data.extend(chunk)
   if marker in data:return
 raise RuntimeError('Timed out waiting for '+repr(marker))
until('⊙> '.encode())
data.clear();payload=(command+'\n').encode()
# Drain echo while writing so neither side fills its PTY buffer.
for offset in range(0,len(payload),512):
 os.write(master,payload[offset:offset+512])
 while select.select([master],[],[],0)[0]:data.extend(os.read(master,65536))
until('⊙> '.encode())
first=bytes(data);data.clear()
os.write(master,b'\x1b[A\n')
until('⊙> '.encode());history=bytes(data)
os.write(master,b'quit\n');p.wait(timeout=10)
restored=termios.tcgetattr(slave)==before
os.close(master);os.close(slave)
label=sys.argv[1] if len(sys.argv)>1 else 'result'
Path(f'measurements/long_numeral_input/{label}.log').write_bytes(first[-65536:]+b'\nHISTORY\n'+history[-65536:])
report={'command_bytes':len(command.encode()),'first_accepted':b'Error: expected' not in first and 'Γ(Λ(D)) == D: true'.encode() in first,'history_accepted':b'Error: expected' not in history and 'Γ(Λ(D)) == D: true'.encode() in history,'terminal_restored':restored}
# Reports use the runtime's actual successful deinterlace label.
report['first_tail']=first.decode(errors='replace')[-200:];report['history_tail']=history.decode(errors='replace')[-200:]
Path(f'measurements/long_numeral_input/{label}.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report,ensure_ascii=False))
assert report['first_accepted'] and report['history_accepted'] and restored,report
assert ('decode(D) = '+n).encode() in first and ('decode(D) = '+n).encode() in history
