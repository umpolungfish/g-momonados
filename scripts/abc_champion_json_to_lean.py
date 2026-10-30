#!/usr/bin/env python3
"""Turn a native ABC champion JSON trace into a Lean trilattice certificate."""
import argparse
import json
from pathlib import Path


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("champions_json", type=Path)
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()
    line = next((s for s in args.champions_json.read_text().splitlines()
                 if s.lstrip().startswith("{")), None)
    if line is None:
        ap.error("no JSON object found")
    doc = json.loads(line)
    events = doc.get("events", [])
    if any(e.get("state") != "B" or e.get("truth") is not True
           or e.get("falsehood") is not True for e in events):
        ap.error("every champion event must carry trilattice state B with both lanes")
    rows = []
    for e in events:
        t = e["triple"]
        rows.append(f"{{ cutoff := {int(e['cutoff'])}, a := {int(t['a'])}, "
                    f"b := {int(t['b'])}, c := {int(t['c'])}, state := .B }}")
    body = ",\n  ".join(rows)
    text = f'''import Imscribing.Paraconsistent.DialetheicWitness

namespace Imscribing.ABC

inductive TrilatticeState where
  | N | T | F | B
  deriving DecidableEq, Repr

structure ChampionCertificate where
  cutoff : Nat
  a : Nat
  b : Nat
  c : Nat
  state : TrilatticeState

def nativeChampionTrace : List ChampionCertificate :=
  [ {body} ]

theorem nativeChampionTrace_is_B :
    ∀ event ∈ nativeChampionTrace, event.state = .B := by
  intro event h
  simp [nativeChampionTrace] at h
  aesop

end Imscribing.ABC
'''
    args.output.write_text(text)
    print(json.dumps({"output": str(args.output), "events": len(events),
                      "state": "B", "lanes": ["truth", "falsehood"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
