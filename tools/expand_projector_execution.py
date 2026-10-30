"""Lift the actual projector image and recursively expand its direct call tokens.

The returned words and instruction operands retain separate boundaries. Indirect
calls keep their operands; their targets are never guessed.
"""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--depth", type=int, default=2)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    vox = root / "Vox/target/release/vox"
    binary = args.binary.resolve()
    args.output.mkdir(parents=True, exist_ok=True)
    words_text = subprocess.check_output([str(vox), "word", str(binary)], text=True)
    wiring_text = subprocess.check_output([str(vox), "disasm", str(binary), "--wiring"], text=True)
    (args.output / "functions.tsv").write_text(words_text)
    (args.output / "wiring.tsv").write_text(wiring_text)
    symbols = {}
    for line in subprocess.check_output(["nm", str(binary)], text=True).splitlines():
        row = line.split()
        if len(row) == 3:
            try:
                symbols[int(row[0], 16)] = row[2]
            except ValueError:
                pass
    words = {}
    for line in words_text.splitlines():
        row = line.replace("\\t", "\t").split("\t", 1)
        if len(row) == 2:
            words[int(row[0], 16)] = row[1]
    instructions = {}
    for line in wiring_text.splitlines():
        row = line.split("\t")
        if len(row) != 8 or row[0] == "function":
            continue
        try:
            owner = int(row[0], 16)
        except ValueError:
            continue
        instructions.setdefault(owner, []).append(dict(zip(
            ["function", "address", "length", "token", "instruction", "operands", "target", "fallthrough"], row)))
    entries = []
    for label, suffix in [("advance", "9Projector7advance"),
                          ("contract_product", "13DecisionArena26contract_amplitude_product"),
                          ("contract_family", "13DecisionArena25contract_amplitude_family")]:
        found = [address for address, name in symbols.items() if name.endswith(suffix)]
        if not found and label != "advance":
            continue
        if len(found) != 1:
            raise ValueError(f"expected one {label} symbol, received {found}")
        address = found[0]
        word = words[address]
        (args.output / (label + ".imasmword")).write_text(word + "\n")
        pairs = subprocess.check_output([str(vox), "pairs", word], text=True)
        (args.output / (label + ".pairs")).write_text(pairs)
        entries.append({"operation": label, "address": hex(address), "symbol": symbols[address]})
    if len(entries) < 2:
        raise ValueError("the image has no retained contraction entry point")
    pending = [(int(entry["address"], 16), 0) for entry in entries]
    expanded = {}
    while pending:
        address, depth = pending.pop(0)
        if hex(address) in expanded:
            continue
        body = instructions.get(address, [])
        expanded[hex(address)] = {"symbol": symbols.get(address), "structure_word": words.get(address),
                                  "expansion_depth": depth, "instructions": body}
        for item in body:
            if item["target"]:
                target = int(item["target"], 16)
                item["target_symbol"] = symbols.get(target)
                if depth < args.depth and item["instruction"].startswith("call") and target in instructions:
                    pending.append((target, depth + 1))
    result = {"binary": str(binary), "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "scope": "static instruction graph with direct call expansion",
              "call_expansion_depth": args.depth, "entry_operations": entries, "functions": expanded}
    (args.output / "recursive_execution.json").write_text(json.dumps(result, ensure_ascii=False, indent=2))
    print(json.dumps({"binary_sha256": result["binary_sha256"], "expanded_functions": len(expanded)}))


if __name__ == "__main__":
    main()
