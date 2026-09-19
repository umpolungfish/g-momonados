#!/usr/bin/env python3
"""
STAGE 78 PROBE v2 — FULL SIXTEEN_3 WITNESS DISCOVERY

Why v2:
  In this environment `vox sixteen3 check ...` is routed through `qr3`.
  `qr3` is a shell function/alias, not an executable visible to subprocess.

Modes
-----
Automatic shell-router mode (recommended):
    python3 tools/probe_78_v2.py

If the shell router cannot be inherited, emit a qr3 batch:
    python3 tools/probe_78_v2.py --emit stage78_commands.txt
    qr3 "$(cat stage78_commands.txt)" > stage78_raw.txt
    python3 tools/probe_78_v2.py --parse stage78_raw.txt

Search deeper:
    python3 tools/probe_78_v2.py --max-len 4
"""

import argparse
import itertools
import os
import re
import shlex
import subprocess
import sys
from collections import OrderedDict

STEP_RE = re.compile(r"^\s*\d+\s+(\S+)\s+\S+\s+(\S+)\s+→\s+(\S+)\s*$")
WORD_RE = re.compile(r"^Word:\s*(\S+)\s*$")
FINAL_RE = re.compile(r"^\s*Final register:\s*(\S+)")
VERDICT_RE = re.compile(r"^\s*Tri-ancestral verdict:\s*(\S+)")

def payloads(alphabet, max_len):
    yield ""
    for n in range(1, max_len + 1):
        for tup in itertools.product(alphabet, repeat=n):
            yield "".join(tup)

def commands_for_payload(payload):
    base = f"⊢∈{payload}∋⊣"
    refined = f"⊢∈{payload}≺∋⊣"
    return (
        f"vox sixteen3 check {base}",
        f"vox sixteen3 check {refined}",
    )

def parse_blocks(text):
    blocks = OrderedDict()
    current = None
    lines = []
    def flush():
        nonlocal current, lines
        if current is not None:
            steps = []
            final = verdict = None
            for line in lines:
                m = STEP_RE.match(line)
                if m:
                    steps.append(m.groups())
                m = FINAL_RE.match(line)
                if m:
                    final = m.group(1)
                m = VERDICT_RE.match(line)
                if m:
                    verdict = m.group(1)
            blocks[current] = {
                "word": current,
                "steps": steps,
                "final": final,
                "verdict": verdict,
                "raw": "\n".join(lines),
            }
        current = None
        lines = []

    for line in text.splitlines():
        m = WORD_RE.match(line)
        if m:
            flush()
            current = m.group(1)
            lines = [line]
        elif current is not None:
            lines.append(line)
    flush()
    return blocks

def run_qr3(command):
    # `bash -ic` sources the user's interactive shell configuration, where
    # qr3 is typically defined as a function/alias.
    env = os.environ.copy()
    env["STAGE78_Q"] = command
    p = subprocess.run(
        ["bash", "-ic", 'qr3 "$STAGE78_Q"'],
        text=True,
        capture_output=True,
        env=env,
    )
    text = (p.stdout or "") + (p.stderr or "")
    if p.returncode != 0 or "Word:" not in text:
        raise RuntimeError(
            "interactive-shell qr3 invocation failed\n"
            f"command: {command}\n"
            f"return code: {p.returncode}\n{text}"
        )
    blocks = parse_blocks(text)
    if len(blocks) != 1:
        raise RuntimeError(f"expected one parsed block, got {len(blocks)}\n{text}")
    return next(iter(blocks.values()))

def register_before_fuse(result):
    for glyph, before, after in result["steps"]:
        if glyph == "∋":
            return before
    raise RuntimeError(f"no FFUSE3 step parsed for {result['word']}")

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

def payload_from_word(word):
    if not (word.startswith("⊢∈") and word.endswith("∋⊣")):
        return None, False
    middle = word[2:-2]
    if middle.endswith("≺"):
        return middle[:-1], True
    return middle, False

def analyze_pairs(pairs, tested):
    witnesses = OrderedDict()
    for payload, base, refined in pairs:
        if base is None:
            continue
        state = register_before_fuse(base)
        if state not in witnesses:
            witnesses[state] = payload

    print("STAGE 78 PROBE — FULL SIXTEEN_3 WITNESS DISCOVERY")
    print("=" * 78)
    for state, payload in witnesses.items():
        print(f"discovered state {state:>8}  payload={payload or '∅'}")
    print()
    print(f"searched {tested} payload(s); discovered {len(witnesses)} state(s)")
    print()

    by_payload = {p: (b, r) for p, b, r in pairs}
    rows = []
    all_roundtrip = True
    for state, payload in witnesses.items():
        base, refined = by_payload[payload]
        pre = register_before_fuse(base)
        at = arev_transition(refined)
        ft = fuse_transition(refined)
        restored = ft[1] if ft else None
        ok = restored == pre
        all_roundtrip &= ok
        rows.append((state, payload or "∅", at, ft, ok, refined["verdict"]))

    print("ROUNDTRIP TABLE")
    print("-" * 78)
    for state, payload, at, ft, ok, verdict in rows:
        arev_s = f"{at[0]}→{at[1]}" if at else "?"
        fuse_s = f"{ft[0]}→{ft[1]}" if ft else "?"
        print(
            f"{state:>8}  payload={payload:<10} "
            f"AREV {arev_s:<15} FFUSE3 {fuse_s:<15} "
            f"restore={str(ok):<5} verdict={verdict}"
        )

    lower = [s for s in witnesses if any(ch in s for ch in ("t", "f"))]
    print()
    print("SUMMARY")
    print("  distinct states discovered :", len(witnesses))
    print("  t/f-bearing states found   :", len(lower))
    print("  all discovered roundtrips  :", all_roundtrip)
    print()
    if len(witnesses) == 16 and all_roundtrip:
        print("STAGE 78 CANDIDATE RESULT : TRUE")
    else:
        print("STAGE 78 CANDIDATE RESULT : OPEN")
        if len(witnesses) < 16:
            print("  carrier coverage incomplete: broaden alphabet and/or depth")
        if not all_roundtrip:
            print("  at least one discovered state obstructs AREV→FFUSE3 restoration")
    return 0

def analyze_raw(text):
    blocks = parse_blocks(text)
    grouped = {}
    for word, result in blocks.items():
        payload, refined = payload_from_word(word)
        if payload is None:
            continue
        grouped.setdefault(payload, [None, None])[1 if refined else 0] = result

    pairs = []
    for payload, (base, refined) in grouped.items():
        if base is not None and refined is not None:
            pairs.append((payload, base, refined))
    if not pairs:
        raise RuntimeError("no complete base/refined word pairs found in raw output")
    return analyze_pairs(pairs, len(pairs))

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--max-len", type=int, default=3)
    ap.add_argument("--alphabet", default="⊤⊥⊞≻≺⋈⊙⊡")
    ap.add_argument("--target", type=int, default=16)
    ap.add_argument("--emit", metavar="FILE")
    ap.add_argument("--parse", metavar="FILE")
    args = ap.parse_args()

    if args.parse:
        return analyze_raw(Path(args.parse).read_text())

    plist = list(payloads(args.alphabet, args.max_len))

    if args.emit:
        lines = []
        for payload in plist:
            lines.extend(commands_for_payload(payload))
        Path(args.emit).write_text("\n".join(lines) + "\n")
        print(f"wrote {len(lines)} qr3 command(s) to {args.emit}")
        print(f'run: qr3 "$(cat {shlex.quote(args.emit)})" > stage78_raw.txt')
        print("then: python3 tools/probe_78_v2.py --parse stage78_raw.txt")
        return 0

    pairs = []
    for i, payload in enumerate(plist, 1):
        base_cmd, refined_cmd = commands_for_payload(payload)
        try:
            base = run_qr3(base_cmd)
            refined = run_qr3(refined_cmd)
        except RuntimeError as e:
            print(e, file=sys.stderr)
            print("\nFALLBACK:", file=sys.stderr)
            print("  python3 tools/probe_78_v2.py --emit stage78_commands.txt", file=sys.stderr)
            print('  qr3 "$(cat stage78_commands.txt)" > stage78_raw.txt', file=sys.stderr)
            print("  python3 tools/probe_78_v2.py --parse stage78_raw.txt", file=sys.stderr)
            return 2
        pairs.append((payload, base, refined))

        # We can stop once all target states have appeared among base words.
        states = {register_before_fuse(b) for _, b, _ in pairs}
        if len(states) >= args.target:
            break

    return analyze_pairs(pairs, len(pairs))

if __name__ == "__main__":
    from pathlib import Path
    raise SystemExit(main())
