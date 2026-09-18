"""Render the implemented factor-relation nesting with ob3ect's circuit tools."""
from pathlib import Path
import sys

ROOT=Path(__file__).resolve().parent
IMSGCT=ROOT.parent
sys.path.insert(0,str(IMSGCT/"IMSCRIBr"))
sys.path.insert(0,str(IMSGCT/"ob3ect"))

from tokens import Token
from wiring import imscr_wiring
from symbolic_diagram import render_wiring_ascii,render_wiring_svg_v3
from proof_scaffold import ouroboricity_tier
from topology import analyze_topology

# Canonical k=0 word from fully_nested_phase_based_prime_factorization.
FACTOR=["VINIT","FSPLIT","AFWD","EVALT","AREV","EVALF","CLINK",
        "IMSCRIB","ENGAGR","FFUSE","IFIX","CLINK","IMSCRIB","TANCH"]

def enfold(depth:int)->list[str]:
    """Put the next factor vessel at the F-arm IMSCRIB site."""
    word=FACTOR[:]
    for _ in range(1,depth):
        word=FACTOR[:7]+word+FACTOR[8:]
    return word

def main()->None:
    ops=enfold(3)
    graph=imscr_wiring(tuple(Token[op] for op in ops))
    graph.name="current_factor_relation_nesting"
    graph.description="factor vessel inside factor vessel; terminal surface after fixation"
    out=ROOT/"measurements"
    (out/"current_factor_relation_nesting.txt").write_text(
        render_wiring_ascii(graph,graph.name)+"\n")
    topology=analyze_topology(ops)
    topo=topology.to_dict() if hasattr(topology,"to_dict") else topology
    svg=render_wiring_svg_v3(graph,graph.name,ouroboricity_tier(ops),
        graph.description,"",pen_mode=True,topology_report=topo)
    svg.save(out/"current_factor_relation_nesting.svg")
    print(f"word operators={len(ops)} nesting_depth={topo.get('nesting_depth')} pairs={topo.get('total_pairs')}")

if __name__=="__main__":
    main()
