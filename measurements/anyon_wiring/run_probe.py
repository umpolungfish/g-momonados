#!/usr/bin/env python3
"""Rebuild the canonical Shor routing probe and its native wiring atlas."""
import hashlib
import html
import json
import re
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def run(args):
    return subprocess.run([str(x) for x in args], check=True,
                          capture_output=True, text=True).stdout


def export_function(atlas, name, destination):
    graph = json.loads((atlas / "circuit.json").read_text())
    function = next(f for f in graph["functions"]
                    if any(name in n for n in f["names"]))
    viewer = (ROOT / "membrane_diagram_viewer.html").read_text()
    script = re.search(r"<script>([\s\S]*?)</script>", viewer).group(1)
    script = script.replace("__DATA__", json.dumps(graph))
    dom = """
const elements={};
function element(){return {options:[],_value:'',innerHTML:'',textContent:'',
 get value(){return this._value||(this.options[0]?.value||'')},
 set value(v){this._value=v},replaceChildren(){this.options=[];this._value=''},
 append(o){this.options.push(o)}}}
const document={getElementById(id){return elements[id]||(elements[id]=element())},
 createElement(){return element()},querySelectorAll(){return []}};
const location={search:''},window={scrollTo(){}};
"""
    result = subprocess.run(["node"], check=True, capture_output=True, text=True,
                            input=dom + script + "\nselect.value=" +
                            json.dumps(function["id"]) +
                            ";draw();console.log(document.getElementById('canvas').innerHTML);")
    destination.write_text(result.stdout)
    return function["id"]


def main():
    source = HERE / "shor_routing_probe.rs"
    kernel = ROOT / "G-mOMonadOS/src/fibonacci_shor.rs"
    binary = HERE / "shor_routing_probe"
    tests = HERE / "shor_constructor_tests"
    run(["rustc", "+stable", "--edition=2021", "-C", "debuginfo=1", "-C",
         "opt-level=0", source, "-o", binary])
    run(["rustc", "+stable", "--edition=2021", "--test", source, "-o", tests])
    (HERE / "constructor_tests.log").write_text(run([tests]))
    reading = run([binary])
    (HERE / "shor_routing.jsonl").write_text(reading)
    rows = [json.loads(line) for line in reading.splitlines()]
    parts = ['<svg xmlns="http://www.w3.org/2000/svg" width="960" height="290">',
             '<rect width="960" height="290" fill="white"/>',
             '<g font-family="monospace" font-size="16" fill="#142033">',
             '<text x="25" y="30">Controlled-phase routing: canonical constructor output</text>']
    for index, row in enumerate(rows[:2]):
        y = 80 + index * 95
        support = sorted({strand for g in row['word']
                          for strand in [abs(g), abs(g) + 1]})
        requested = f"control {row['control']} to target {row['target']}"
        emitted = "crossed strands " + ", ".join(map(str, support))
        parts.extend([f'<text x="25" y="{y}">{html.escape(requested)}</text>',
                      f'<path d="M310 {y-6} L460 {y-6}" stroke="#a84232" stroke-width="3"/>',
                      f'<text x="480" y="{y}">{html.escape(emitted)}</text>',
                      f'<text x="25" y="{y+30}" font-size="14">word {html.escape(str(row["word"]))}</text>'])
    parts.append('<text x="25" y="270">Bases 2 and 8 modulo 15 emit the same complete braid.</text></g></svg>')
    figure = HERE / 'routing.svg'
    figure.write_text(''.join(parts))
    run(['rsvg-convert', '-f', 'pdf', '-o', HERE / 'routing.pdf', figure])
    print(reading, end="")
    print(run(["python3", ROOT / "render_membrane_diagram.py", binary,
               "--out", HERE / "shor_wiring"]), end="")
    address = export_function(HERE / "shor_wiring", "controlled_phase_braid",
                              HERE / "controlled_phase.svg")
    manifest = {
        "compiler": run(["rustc", "+stable", "--version"]).strip(),
        "kernel": str(kernel),
        "kernel_sha256": hashlib.sha256(kernel.read_bytes()).hexdigest(),
        "probe_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "controlled_phase_function": address,
        "target_change_preserved": rows[3]["target_change_preserved"],
        "control_change_preserved": rows[3]["control_change_preserved"],
        "base_change_preserved_in_full_braid": rows[-1]["base_change_preserved_in_full_braid"],
        "expected_work_images_of_one": {"base_2_mod_15": 2, "base_8_mod_15": 8},
    }
    (HERE / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")


if __name__ == "__main__":
    main()
