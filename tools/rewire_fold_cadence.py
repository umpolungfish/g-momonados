"""Rewire the compiled nested-operation fold gate from eight operations to one.

The gate must increment and store the same object field, compare it with eight,
and call that object's fold method on equality. The fold method must reset the
same field. Only the checked immediate byte in a copied ELF is changed.
"""
import argparse
import hashlib
import io
import json
import re
import stat
from pathlib import Path

from elftools.elf.elffile import ELFFile


def patch(elf_path, tape_path, output, manifest, cadence=1):
    if not 1 <= cadence <= 8:
        raise ValueError("cadence must stay within the original eight-operation interval")
    if output.resolve() == elf_path.resolve():
        raise ValueError("the original ELF must remain preserved")
    raw = elf_path.read_bytes()
    elf = ELFFile(io.BytesIO(raw))
    symbols = list(elf.get_section_by_name(".symtab").iter_symbols())
    def method(suffix):
        matches = [s for s in symbols if "ququart_folded_work" in s.name
                   and s.name.endswith(suffix) and s["st_size"]]
        if len(matches) != 1:
            raise ValueError(f"expected one compiled {suffix} method")
        s = matches[0]
        return s.name, s["st_value"], s["st_value"] + s["st_size"]
    apply_name, start, end = method("12apply_nested")
    fold_name, fold, fold_end = method("4fold")
    instructions = []
    address = None
    tape = tape_path.read_text()
    if not tape.startswith("; ⊙ module"):
        raise ValueError("expected a Vox executable module")
    for line in tape.splitlines():
        if line.startswith("@0x"):
            address = int(line[1:], 16)
        elif address is not None and "\t" in line and not line.startswith(";"):
            instructions.append((address, line.split("\t")))
    body = [(a, f) for a, f in instructions if start <= a < end]
    candidates = []
    for i in range(3, len(body) - 4):
        address, fields = body[i]
        if fields != ["⊤", "cmp", "r:rax", "i:0x8"]:
            continue
        load, increment, store = [body[j][1] for j in (i - 3, i - 2, i - 1)]
        branch, argument, call = [body[j][1] for j in (i + 1, i + 2, i + 3)]
        if len(load) != 4 or load[:3] != ["⋈", "mov", "r:rax"]:
            continue
        match = re.fullmatch(r"m:([^:]+)::1:(0x[0-9a-f]+):8", load[3])
        if not match:
            continue
        base, offset = match.groups()
        if increment != ["⊞", "inc", "r:rax"] or store != ["⋈", "mov", load[3], "r:rax"]:
            continue
        if branch != ["∈", "ne", f"i:0x{body[i + 4][0]:x}"]:
            continue
        if argument != ["⋈", "mov", "r:rdi", "r:" + base]:
            continue
        if call != ["≻", "call", f"i:0x{fold:x}"]:
            continue
        resets = [a for a, f in instructions if fold <= a < fold_end and len(f) == 4
                  and f[:2] == ["⋈", "mov"] and f[3] == "i:0x0"
                  and re.fullmatch(r"m:[^:]+::1:" + re.escape(offset) + r":8", f[2])]
        if len(resets) == 1:
            candidates.append((address, load[3], body[i + 3][0], resets[0]))
    if len(candidates) != 1:
        raise ValueError(f"expected one verified fold gate, found {len(candidates)}")
    address, counter, call_address, reset = candidates[0]
    def file_offset(pc, size):
        segments = [s for s in elf.iter_segments() if s["p_type"] == "PT_LOAD"
                    and s["p_vaddr"] <= pc and pc + size <= s["p_vaddr"] + s["p_filesz"]]
        if len(segments) != 1:
            raise ValueError("instruction must occupy one file-backed load segment")
        s = segments[0]
        return s["p_offset"] + pc - s["p_vaddr"]
    at = file_offset(address, 4)
    if raw[at:at + 4] != bytes.fromhex("4883f808"):
        raise ValueError("fold compare machine bytes differ from the decoded gate")
    call_at = file_offset(call_address, 5)
    if raw[call_at] != 0xe8 or call_address + 5 + int.from_bytes(raw[call_at + 1:call_at + 5], "little", signed=True) != fold:
        raise ValueError("machine call does not reach the checked fold method")
    patched = raw[:at + 3] + bytes([cadence]) + raw[at + 4:]
    changed_bytes = sum(a != b for a, b in zip(raw, patched))
    assert changed_bytes == int(cadence != 8)
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists() and output.read_bytes() != patched:
        raise ValueError("output already contains a different artifact")
    output.write_bytes(patched)
    output.chmod(stat.S_IMODE(elf_path.stat().st_mode))
    result = {
        "original_elf": str(elf_path), "patched_elf": str(output),
        "original_sha256": hashlib.sha256(raw).hexdigest(),
        "patched_sha256": hashlib.sha256(patched).hexdigest(),
        "source_tape_sha256": hashlib.sha256(tape.encode()).hexdigest(),
        "method": apply_name, "fold_method": fold_name,
        "gate_address": hex(address), "counter_operand": counter,
        "reset_address": hex(reset), "changed_file_offset": at + 3,
        "original_cadence": 8, "patched_cadence": cadence, "changed_bytes": changed_bytes,
        "effect": f"Pin current roots and release unreachable nodes after every {cadence} completed nested operations.",
        "semantic_validation": "Pending independent readout comparisons; a control-flow closure check alone does not prove value equivalence.",
    }
    manifest.write_text(json.dumps(result, indent=2) + "\n")
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(__doc__)
    for name in ["elf", "source-imasm", "output", "manifest"]:
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--cadence", type=int, default=1)
    args = parser.parse_args()
    print(json.dumps(patch(args.elf, args.source_imasm, args.output, args.manifest, args.cadence)))
