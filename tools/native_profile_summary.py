"""Summarize completed native samples against exact ELF function ranges."""
import argparse
import bisect
import collections
import csv
import json
from pathlib import Path

from elftools.elf.elffile import ELFFile


def summarize(elf_path, prefix):
    with open(elf_path, "rb") as binary:
        elf = ELFFile(binary)
        functions = sorted(
            (s["st_value"], s["st_size"], s.name)
            for s in elf.get_section_by_name(".symtab").iter_symbols()
            if s["st_info"]["type"] == "STT_FUNC" and s["st_size"]
        )
    starts = [f[0] for f in functions]
    transitions = []
    latest = None
    with open(str(prefix) + ".ququart.tsv") as stream:
        for row in csv.DictReader(stream, delimiter="\t"):
            try:
                values = list(row.values())
                seconds = float(values[0])
                phase = (int(values[7]), int(values[3]))
            except (TypeError, ValueError, IndexError):
                continue  # A live writer may have an incomplete final row.
            latest = row
            if not transitions or transitions[-1][1] != phase:
                transitions.append((seconds, phase))
    times = [t for t, _ in transitions]
    counts = collections.defaultdict(collections.Counter)
    bounds = {}
    with open(str(prefix) + ".samples.tsv") as stream:
        for row in csv.DictReader(stream, delimiter="\t"):
            try:
                seconds = float(row["seconds"])
                address = int(row["address"], 16)
            except (TypeError, ValueError):
                continue
            phase_index = bisect.bisect_right(times, seconds) - 1
            phase = transitions[phase_index][1] if phase_index >= 0 else (0, 0)
            index = bisect.bisect_right(starts, address) - 1
            name = "unmapped"
            if index >= 0:
                start, size, symbol = functions[index]
                if address < start + size:
                    name = symbol
            counts[phase][name] += 1
            bounds.setdefault(phase, [seconds, seconds])[1] = seconds
    return {
        "elf": str(elf_path), "profile": str(prefix), "latest_counters": latest,
        "scope": "Stopped instruction samples, grouped by last observed shot and digit; no inferred factors.",
        "phases": [{
            "shot_started": phase[0], "measured_digits": phase[1],
            "sample_seconds": bounds[phase], "samples": sum(counter.values()),
            "top_symbols": counter.most_common(12),
        } for phase, counter in sorted(counts.items())],
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("elf", type=Path)
    parser.add_argument("prefix", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    args.output.write_text(json.dumps(summarize(args.elf, args.prefix), indent=2) + "\n")
