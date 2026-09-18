#!/usr/bin/env python3
"""imasm_compile_all.py — compile every instruction word in every imasm/*.imasm
file to a real standalone binary under bin/, mirroring imasm/'s layout.

A .imasm file is a saved module in the annotated format the emitter writes:

    ; comment              <- skip
    @0xADDR                <- address label, skip
    GLYPH \t field \t ...  <- one instruction, compile GLYPH as the word

Regenerate this after any change to imasm/ or imasm_emit.py; bin/ is
gitignored, it's build output, not source.
"""
import glob, os, sys, time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from imasm_emit import compile_word

HERE = os.path.dirname(os.path.abspath(__file__))
SRC_ROOT = os.path.join(HERE, "imasm")
OUT_ROOT = os.path.join(HERE, "bin")

# The twelve marks. A line whose first field is one of these is an instruction;
# anything else (`;` comment, `@` label, stray continuation, empty) is skipped.
GLYPHS = frozenset("⊢⊣≻≺⋈⊤∈∋⊙⊥⊞⊡")


def instruction_word(line):
    """Return the glyph word for an instruction line, or None if the line is
    an annotation, a blank, or anything that isn't a single leading glyph."""
    s = line.strip()
    if not s:
        return None
    if s[0] in ";@":
        return None
    head = s.split("\t", 1)[0].strip()
    if len(head) == 1 and head in GLYPHS:
        return head
    return None


def main():
    files = sorted(glob.glob(os.path.join(SRC_ROOT, "**", "*.imasm"), recursive=True))
    total_lines = 0
    compiled = 0
    skipped = 0
    failed = []
    t0 = time.time()

    for fp in files:
        rel = os.path.relpath(fp, SRC_ROOT)
        modname = rel[:-len(".imasm")]
        outdir = os.path.join(OUT_ROOT, modname)
        os.makedirs(outdir, exist_ok=True)
        # Index by non-blank line position, matching the previous layout so
        # existing tooling that reads bin/<modname>/<i> still finds the same
        # instruction at the same index.
        i = 0
        for raw in open(fp, encoding="utf-8"):
            line = raw.rstrip("\n")
            if not line.strip():
                continue
            i += 1
            total_lines += 1
            word = instruction_word(line)
            if word is None:
                skipped += 1
                continue
            outpath = os.path.join(outdir, str(i))
            try:
                compile_word(word, outpath)
                compiled += 1
            except Exception as e:
                failed.append((fp, i, f"{e} (word={word!r})"))

    dt = time.time() - t0
    print(f"files: {len(files)}  lines: {total_lines}  compiled: {compiled}  "
          f"skipped: {skipped}  failed: {len(failed)}  time: {dt:.1f}s")
    for fp, i, e in failed[:20]:
        print("FAIL", fp, i, e)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())