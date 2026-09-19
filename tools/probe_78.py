#!/usr/bin/env python3
"""
STAGE 78 PROBE — FULL SIXTEEN_3 WITNESS DISCOVERY / AREV-FFUSE3 ROUNDTRIP

Run inside G-mOMonadOS.  The probe does not assume which payloads create the
lowercase t/f lanes.  It searches short payloads inside one FSPLIT3 frame,
records the visible register immediately before FFUSE3, then inserts AREV
immediately before FFUSE3 and tests whether fusion restores that state.

Default invocation:
    python3 tools/probe_78.py

If direct `vox sixteen3 check ...` is unavailable but qr3 works:
    python3 tools/probe_78.py --via-qr3

Increase search if fewer than 16 states are found:
    python3 tools/probe_78.py --max-len 4
"""

import argparse
import itertools
import re
import subprocess
import sys
from collections import OrderedDict

STEP_RE = re.compile(
    r"^\s*\d+\s+(\S+)\s+\S+\s+(\S+)\s+→\s+(\S+)\s*$"
)
FINAL_RE = re.compile(r"^\s*Final register:\s*(\S+)")
VERDICT_RE = re.compile(r"^\s*Tri-ancestral verdict:\s*(\S+)")

def run_check(word, via_qr3=False):
    if via_qr3:
        cmd = ["qr3", f"vox sixteen3 check {word}"]
    else:
        cmd = ["vox", "sixteen3", "check", word]
    p = subprocess.run(cmd, text=True, capture_output=True)
    text = (p.stdout or "") + (p.stderr or "")
    if p.returncode != 0:
        raise RuntimeError(f"command failed ({p.returncode}): {' '.join(cmd)}\n{text}")
    steps = []
    final = verdict = None
    for line in text.splitlines():
        m = STEP_RE.match(line)
        if m:
            glyph, before, after = m.groups()
            steps.append((glyph, before, after))
        m = FINAL_RE.match(line)
        if m:
            final = m.group(1)
        m = VERDICT_RE.match(line)
        if m:
            verdict = m.group(1)
    if not steps or final is None:
        raise RuntimeError(f"could not parse output for {word!r}\n{text}")
    return {"word": word, "steps": steps, "final": final, "verdict": verdict, "raw": text}

def register_before_fuse(result):
    for i, (glyph, before, after) in enumerate(result["steps"]):
        if glyph == "∋":
            return before, i
    raise RuntimeError("no FFUSE3 step found")

def arev_transition(result):
    for glyph, before, after in result["steps"]:
        if glyph == "≺":
            return before, after
    return None

def fuse_transition(result):
    for glyph, before, after in result["steps"]:
        if glyph == "∋":
            return before, after
    return None

def payloads(alphabet, max_len):
    yield ""
    for n in range(1, max_len + 1):
        for tup in itertools.product(alphabet, repeat=n):
            yield "".join(tup)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--max-len", type=int, default=3)
    ap.add_argument(
        "--alphabet",
        default="⊤⊥⊞≻≺⋈⊙⊡",
        help="payload glyph alphabet; structural ∈/∋ are supplied by the probe",
    )
    ap.add_argument("--via-qr3", action="store_true")
    ap.add_argument("--target", type=int, default=16,
                    help="stop after this many distinct visible registers")
    args = ap.parse_args()

    witnesses = OrderedDict()
    tested = 0

    print("STAGE 78 PROBE — FULL SIXTEEN_3 WITNESS DISCOVERY")
    print("=" * 78)
    print("alphabet:", " ".join(args.alphabet))
    print("max payload length:", args.max_len)
    print()

    for payload in payloads(args.alphabet, args.max_len):
        base = f"⊢∈{payload}∋⊣"
        try:
            r = run_check(base, args.via_qr3)
        except RuntimeError as e:
            print(e, file=sys.stderr)
            return 2
        tested += 1
        state, _ = register_before_fuse(r)
        if state not in witnesses:
            witnesses[state] = payload
            print(f"discovered state {state:>6}  payload={payload or '∅'}")
            if len(witnesses) >= args.target:
                break

    print()
    print(f"searched {tested} payload(s); discovered {len(witnesses)} state(s)")
    print()

    rows = []
    all_roundtrip = True
    for state, payload in witnesses.items():
        base = f"⊢∈{payload}∋⊣"
        refined = f"⊢∈{payload}≺∋⊣"
        rb = run_check(base, args.via_qr3)
        rr = run_check(refined, args.via_qr3)

        pre, _ = register_before_fuse(rb)
        at = arev_transition(rr)
        ft = fuse_transition(rr)
        restored = ft[1] if ft else None
        ok = restored == pre
        all_roundtrip &= ok
        rows.append((state, payload or "∅", at, ft, restored, ok, rr["verdict"]))

    print("ROUNDTRIP TABLE")
    print("-" * 78)
    for state, payload, at, ft, restored, ok, verdict in rows:
        arev_s = f"{at[0]}→{at[1]}" if at else "?"
        fuse_s = f"{ft[0]}→{ft[1]}" if ft else "?"
        print(
            f"{state:>6}  payload={payload:<8} "
            f"AREV {arev_s:<12} FFUSE3 {fuse_s:<12} "
            f"restore={str(ok):<5} verdict={verdict}"
        )

    lower = [s for s in witnesses if any(ch in s for ch in ("t", "f"))]
    print()
    print("SUMMARY")
    print("  distinct states discovered :", len(witnesses))
    print("  t/f-bearing states found   :", len(lower))
    print("  all discovered roundtrips  :", all_roundtrip)

    if len(witnesses) == 16 and all_roundtrip:
        print()
        print("STAGE 78 CANDIDATE RESULT : TRUE")
        print("  full discovered SIXTEEN_3 carrier restored by framed AREV→FFUSE3")
        return 0

    print()
    print("STAGE 78 CANDIDATE RESULT : OPEN")
    if len(witnesses) < 16:
        print("  increase --max-len and/or broaden --alphabet until carrier coverage is resolved")
    if not all_roundtrip:
        print("  at least one discovered state is an AREV→FFUSE3 obstruction")
    return 1

if __name__ == "__main__":
    raise SystemExit(main())
