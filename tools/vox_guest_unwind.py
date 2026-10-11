"""Unwind a Vox guest using its baked ELF's call-frame information."""
import bisect
from elftools.elf.elffile import ELFFile
from elftools.dwarf.callframe import FDE


def unwind(path, registers, pc, read_memory, limit=256):
    names = ["rax", "rdx", "rcx", "rbx", "rsi", "rdi", "rbp", "rsp",
             "r8", "r9", "r10", "r11", "r12", "r13", "r14", "r15"]
    regs = {i: registers[name] for i, name in enumerate(names)}
    regs[16] = pc
    frames = []
    with open(path, "rb") as binary:
        elf = ELFFile(binary)
        entries = sorted((entry for entry in elf.get_dwarf_info().EH_CFI_entries()
                          if isinstance(entry, FDE)), key=lambda entry: entry["initial_location"])
        starts = [entry["initial_location"] for entry in entries]
        symbols = sorted((symbol["st_value"], symbol.name)
                         for symbol in elf.get_section_by_name(".symtab").iter_symbols()
                         if symbol["st_info"]["type"] == "STT_FUNC")
        symbol_starts = [address for address, _ in symbols]
        for frame in range(limit):
            address = regs[16] - bool(frame)
            index = bisect.bisect_right(starts, address) - 1
            if index < 0:
                break
            entry = entries[index]
            if address >= entry["initial_location"] + entry["address_range"]:
                break
            table = entry.get_decoded().table
            row = table[bisect.bisect_right([item["pc"] for item in table], address) - 1]
            if row["cfa"].reg is None:
                break
            cfa = regs[row["cfa"].reg] + row["cfa"].offset
            symbol_index = bisect.bisect_right(symbol_starts, address) - 1
            frames.append({"pc": hex(regs[16]), "cfa": hex(cfa),
                           "symbol": symbols[symbol_index][1] if symbol_index >= 0 else "",
                           "registers": {names[i]: hex(value) for i, value in regs.items() if i < 16}})
            recovered = dict(regs)
            recovered[7] = cfa
            for register, rule in row.items():
                if not isinstance(register, int):
                    continue
                if rule.type == "OFFSET":
                    recovered[register] = int.from_bytes(read_memory(cfa + rule.arg, 8), "little")
                elif rule.type == "REGISTER":
                    recovered[register] = regs[rule.arg]
                elif rule.type == "VAL_OFFSET":
                    recovered[register] = cfa + rule.arg
                elif rule.type == "UNDEFINED":
                    recovered.pop(register, None)
                elif rule.type != "SAME_VALUE":
                    raise ValueError(f"unsupported guest unwind rule: {rule}")
            if not recovered.get(16) or recovered[7] <= regs[7]:
                break
            regs = recovered
    return frames
