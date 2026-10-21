from pathlib import Path
import subprocess,json,hashlib
ROOT=Path(__file__).resolve().parents[3]
BASE=Path(__file__).resolve().parent
DOC=ROOT/'ig-docs'
def quote(value):return json.dumps(value,ensure_ascii=False)
lines=['digraph preparation {','graph [rankdir=TB,fontname="DejaVu Sans",label="Native protocol dispatch during N-only preparation",labelloc=t];','node [shape=box,fontname="DejaVu Sans",style="rounded,filled",fillcolor="#eef4fa"];', 'edge [fontname="DejaVu Sans"];', 'input [label="N alone + protocol selection"];','init [label="Prepare shared nontrivial E register\nSource words, residual bank and retained Y"];','input -> init;']
words={};sources=[]
actions={'VINIT':'source-bound entry','IMSCRIB':'retain source words','AFWD':'integer affine rows via retained Y\nretain integer bank; reduce modulo N once','AREV':'native involution + numerical E ↦ N+1−E','CLINK':'resident polynomial commutator + complex SIC recovery','FSPLIT':'complementary complex-coordinate arms + ancestry tag','EVALT':'native truth + residual-zero projection','EVALF':'native falsity + reversed point projection','FFUSE':'select ancestry sectors, clear tag, coherently reunite','ENGAGR':'retain coordinate ancestry; adjoin information flags','IFIX':'projected stability, retained Y and central phase loop','TANCH':'return retained roots and banks'}
for kind,name in [('srfp','self_reading_fixed_point'),('dnn','dual_named_numerals')]:
 path=ROOT/'ob3ect/digital'/name/(name+'_ob3ect.json');sources.append(path)
 definition=json.loads(path.read_text());words[kind]=definition['glyph_word'];w=definition['wiring_check']
 lines.append(f'subgraph cluster_{kind} {{ label={quote(kind.upper()+": "+words[kind])};')
 for i,op in enumerate(w['nodes']):
  color='#ffe8e5' if op=='AFWD' else '#eef4fa'
  label=str(i)+' '+op+'\n'+actions[op]
  lines.append(f'{kind}{i} [label={quote(label)},fillcolor="{color}"];')
 for a,port,b,target in w['wires']:
  lines.append(f'{kind}{a} -> {kind}{b} [label={quote(port+"→"+target)}];')
 lines.append('}');lines.append(f'init -> {kind}0 [label="selected protocol"];');lines.append(f'{kind}11 -> artifact;')
lines+=['artifact [label="Prepared artifact\nShared node pool, every protocol bank, source, Y and traces\nFactor leaves only after exact numerical closure"];','read [label="Read prepared factor leaves\nSeparate load/read/product verification clocks"];','artifact -> read;', 'note [shape=note,label="AFWD still expands the arithmetic relation.\nDispatch and bank recovery are tested separately from\nflat preparation, full tuple predicates and factor production."];','note -> artifact [style=dashed,arrowhead=none];','}']
path=DOC/'conventional_preparation_process_20261020.dot';path.write_text('\n'.join(lines)+'\n')
for ext in ['svg','png']:subprocess.run(['dot','-T'+ext,str(path),'-o',str(path.with_suffix('.'+ext))],check=True)
sources += [ROOT/'G-mOMonadOS/src/conventional_spectrum.rs',ROOT/'G-mOMonadOS/src/conventional_translation.rs',ROOT/'G-mOMonadOS/src/ququart_decision.rs',ROOT/'conventional_translation.py',path]
(BASE/'process_diagram_binding.json').write_text(json.dumps({'words':words,'sources':{str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sources},'scope':'Implemented native graph dispatch and explicit numerical lifts. Full tuple semantics, flat preparation and wide factor production remain separate open results.'},indent=2,ensure_ascii=False)+'\n')
print(path.with_suffix('.svg'))
