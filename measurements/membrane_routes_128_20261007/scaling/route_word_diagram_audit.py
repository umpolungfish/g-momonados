"""Render and verdict the operator words used by the G membrane routes."""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
IMSGCT = ROOT.parent
sys.path[:0] = [
    str(IMSGCT / "IMSCRIBr"),
    str(IMSGCT / "ob3ect"),
    str(IMSGCT / "ob3ect" / "digital"),
]

from imasm16_3_core import Sequence16_3Trace, parse_glyph_word
from proof_scaffold import ouroboricity_tier
from symbolic_diagram import render_wiring_svg_v3
from topology import analyze_topology
from tokens import Token
from wiring import imscr_wiring

SCALE = Path(__file__).resolve().parent
SOURCE = ROOT / "membranes" / "src"
OUTPUT = SCALE / "256"
MARKS = dict(zip(
    "⊢⊣≻≺⋈⊙∈∋⊤⊥⊞⊡",
    ("VINIT", "TANCH", "AFWD", "AREV", "CLINK", "IMSCRIB", "FSPLIT",
     "FFUSE", "EVALT", "EVALF", "ENGAGR", "IFIX"),
))


def constant(path: Path, name: str) -> str:
    source = path.read_text()
    match = re.search(rf'const {name}: &str = "([^"]+)";', source)
    if match is None:
        raise RuntimeError(f"missing {name} in {path}")
    return match.group(1)


def fixed_word(depth: int) -> str:
    path = SOURCE / "bin" / "fixed_nested.rs"
    prefix = constant(path, "FACTOR_PREFIX")
    suffix = constant(path, "FACTOR_SUFFIX")
    word = f"{prefix}⊙{suffix}"
    for _ in range(1, depth):
        word = f"{prefix}{word}{suffix}"
    return word


def measure(name: str, word: str) -> dict:
    ops = [MARKS[mark] for mark in word]
    topology = analyze_topology(ops).to_dict()
    trace = Sequence16_3Trace(parse_glyph_word(word))
    trace.run()
    verdict = trace.json_report()
    graph = imscr_wiring(tuple(Token[op] for op in ops))
    graph.name = name
    graph.description = f"Executable operator word from {name}"
    svg = render_wiring_svg_v3(
        graph,
        graph.name,
        ouroboricity_tier(ops),
        graph.description,
        "",
        pen_mode=True,
        topology_report=topology,
    )
    svg.save(OUTPUT / f"{name}_wiring.svg")
    return {
        "word": word,
        "topology": topology,
        "imasm": {
            "verdict": verdict["verdict"],
            "verdict_reading": verdict["verdict_reading"],
            "closed": verdict["closed"],
        },
        "diagram": f"256/{name}_wiring.svg",
    }


def main() -> None:
    words = {
        "ecm_stage_word": constant(SOURCE / "ecm.rs", "ECM_WORD"),
        "ecm_scalar_flat_control": "⊢≻⋈⊣",
        "ecm_scalar_step": constant(SOURCE / "ecm.rs", "ECM_SCALAR_WORD"),
        "order_cycle_frame": constant(SOURCE / "order_cycle.rs", "ORDER_WORD"),
        "fixed_nested": fixed_word(3),
    }
    report = {name: measure(name, word) for name, word in words.items()}
    report["radix4"] = {
        "operator_word": None,
        "implementation": "factor_radix4 recursively lifts WordTape digit prefixes",
        "diagram": None,
    }
    path = SCALE / "route_word_diagram_audit.json"
    path.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    for name, result in report.items():
        if "imasm" in result:
            print(name, result["imasm"]["verdict"], result["topology"]["total_pairs"], result["diagram"])
        else:
            print(name, "no operator word", result["implementation"])


if __name__ == "__main__":
    main()
