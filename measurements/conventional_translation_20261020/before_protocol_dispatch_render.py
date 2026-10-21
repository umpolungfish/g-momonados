from pathlib import Path
import subprocess,json,hashlib
ROOT=Path(__file__).resolve().parents[3]
BASE=Path(__file__).resolve().parent
DOC=ROOT/'ig-docs'
words={}
for name in ['self_reading_fixed_point','dual_named_numerals']:
 path=ROOT/'ob3ect/digital'/name/(name+'_ob3ect.json')
 word=json.loads(path.read_text())['glyph_word'];words[name]=word
 out=DOC/('conventional_'+name+'_20261020.svg')
 p=subprocess.run(['python3',str(ROOT/'Vox/render_carrier_diagram.py'),word,name,str(out)],capture_output=True,text=True,check=True)
 (BASE/(name+'_diagram.txt')).write_text(p.stdout)
dot=r'''digraph preparation {
 graph [rankdir=TB,bgcolor="#fafbfc",fontname="DejaVu Sans",fontsize=18,label="Current preparation: actual dispatch and carried state",labelloc=t,nodesep=0.35,ranksep=0.45,pad=0.25];
 node [shape=box,style="rounded,filled",fontname="DejaVu Sans",fontsize=12,color="#607486",fillcolor="#eef4fa",margin="0.14,0.10"];
 edge [fontname="DejaVu Sans",fontsize=10,color="#536b80"];
 subgraph cluster_spec {label="Requested source constructions";color="#8899aa";
  srfp [label="SRFP: Y on a multiplicative pairing\nNative word from saved definition",fillcolor="#edf8ee"];
  dnn [label="DNN: N with the two-point spectrum\nNative word from saved definition",fillcolor="#edf8ee"];
  unused [label="Python stores glyph tuples as class attributes.\nNeither native word is dispatched by prepare().",fillcolor="#ffe8e5",color="#bb493a"];
  srfp -> unused [style=dashed,arrowhead=none];dnn -> unused [style=dashed,arrowhead=none];
 }
 input [label="N alone\nCLI / REPL / Python"];
 prepare [label="conventional_spectrum::prepare(N)\nSame numerical path for srfp and dnn"];
 init [label="PointMembrane::prepare\nRetain every E with 2 <= E < N; residual r = 0\nKeep source words and preparation bank"];
 y [label="ResidentY::advance\npowerset bind on the same diagram root"];
 expand [label="arithmetic_step\nFor each bit pair: controlled modular addition\nBuild r(E) = E*E - E modulo N",fillcolor="#ffe8e5",color="#bb493a"];
 intern [label="DecisionArena\npartition / add / intern / reclaim\nLarger partial function means more nodes",fillcolor="#fff0d9",color="#bf8634"];
 filter [label="Only AFTER every arithmetic row:\nretain residual r = 0, then establish stability",fillcolor="#ffe8e5",color="#bb493a"];
 point [label="Sample numerical E from surviving register"];
 closure [label="idempotent_point::close(N,E)\nDerive divisor and quotient; verify exact closure"];
 artifact [label="Prepared artifact\nSource, banks, Y, binary tree and product-tree leaves"];
 read [label="read_prepared\nCopy two evaluated leaves\nThen separately verify their product"];
 input -> prepare -> init -> y -> expand -> intern;
 intern -> y [label="next bit pair",constraint=false];
 intern -> filter [label="all rows complete"];
 filter -> point -> closure -> artifact -> read;
 unused -> prepare [style=dashed,color="#bb493a",label="missing protocol dispatch",arrowhead=none];
 observation [shape=note,label="Measured 896-bit run, stopped:\n339,074 retained nodes at 492.305 s\nrow 2, column 209; projection not reached",fillcolor="#ffffff"];
 intern -> observation [style=dashed,arrowhead=none];
 labelnote [shape=note,label="Source/call diagram plus saved execution observations.\nNative word diagrams show wiring, not factor production.",fillcolor="#ffffff"];
 read -> labelnote [style=invis];
}'''
path=DOC/'conventional_preparation_process_20261020.dot';path.write_text(dot)
for ext in ['svg','png']:
 subprocess.run(['dot','-T'+ext,str(path),'-o',str(path.with_suffix('.'+ext))],check=True)
paths=[ROOT/'G-mOMonadOS/src/conventional_spectrum.rs',ROOT/'G-mOMonadOS/src/conventional_translation.rs',ROOT/'conventional_translation.py',path]
(BASE/'process_diagram_binding.json').write_text(json.dumps({'words':words,'sources':{str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in paths},'scope':'Reviewed source dispatch and native symbolic word wiring, with retained actual896 run observations. No claim of protocol execution or factor completion.'},indent=2,ensure_ascii=False)+'\n')
print(path.with_suffix('.svg'))
