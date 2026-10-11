"""Read a stopped Vox VM's guest progress counters without changing its state.

GDB must stop at Machine::alu entry, where rdi is the Machine pointer. Offsets
are measured from the current Vox test executable's DWARF, retained alongside
the sample in vm_layout.json. Guest counter addresses come from the baked ELF.
"""
import bisect
import gdb
import json
import os
import sys

spec = json.load(open(os.environ["VOX_SNAPSHOT_SPEC"]))
layout = spec["layout"]
inferior = gdb.selected_inferior()
machine = int(gdb.parse_and_eval("$rdi"))


def integer(address, width=8):
    return int.from_bytes(inferior.read_memory(address, width).tobytes(), "little")


def guest_page(key):
    tree = machine + layout["memory"]
    node = integer(tree + layout["root_node"])
    height = integer(tree + layout["root_height"])
    if height > 12:
        raise ValueError("VM page tree height differs from its measured layout")
    while node:
        length = integer(node + layout["node_length"], 2)
        if length > 11:
            raise ValueError("VM page node length differs from its measured layout")
        keys = [integer(node + layout["node_keys"] + 8 * i) for i in range(length)]
        if keys != sorted(keys):
            raise ValueError("VM page keys are not ordered")
        slot = bisect.bisect_left(keys, key)
        if slot < length and keys[slot] == key:
            return integer(node + layout["node_values"] + 8 * slot)
        if height == 0:
            return 0
        node = integer(node + layout["node_edges"] + 8 * slot)
        height -= 1
    return 0


def guest_integer(address):
    value = bytearray()
    for at in range(address, address + 8):
        page = guest_page(at >> 12)
        value.extend(inferior.read_memory(page + (at & 4095), 1).tobytes() if page else b"\0")
    return int.from_bytes(value, "little")


def guest_bytes(address, size):
    value = bytearray()
    while len(value) < size:
        at = address + len(value)
        count = min(4096 - (at & 4095), size - len(value))
        page = guest_page(at >> 12)
        value.extend(inferior.read_memory(page + (at & 4095), count).tobytes() if page else bytes(count))
    return value


result = {
    "pid": inferior.pid,
    "guest_pc": hex(integer(machine + layout["current_pc"])),
    "steps": integer(machine + layout["steps"]),
    "counters": {name: guest_integer(address) for name, address in spec["counters"].items()},
}
if spec.get("capture_page_count"):
    def page_count(node, height):
        if not node:
            return 0
        length = integer(node + layout["node_length"], 2)
        if length > 11 or height > 12:
            raise ValueError("VM page tree differs from its measured layout")
        return length + (sum(page_count(integer(node + layout["node_edges"] + 8 * slot), height - 1)
                             for slot in range(length + 1)) if height else 0)

    tree = machine + layout["memory"]
    result["materialized_guest_pages"] = page_count(integer(tree + layout["root_node"]),
                                                   integer(tree + layout["root_height"]))
    result["materialized_guest_bytes"] = result["materialized_guest_pages"] * 4096
if spec.get("capture_syslog"):
    capacity, pointer = integer(machine), integer(machine + 8)
    head, length = integer(machine + 16), integer(machine + 24)
    if length > 100_000 or length > capacity:
        raise ValueError("VM log differs from its measured deque layout")
    lines = []
    for index in range(length):
        entry = pointer + 24 * ((head + index) % capacity)
        address, size = integer(entry + 8), integer(entry + 16)
        if size > 16384:
            raise ValueError("VM log entry exceeds its measured string layout")
        lines.append(inferior.read_memory(address, size).tobytes().decode())
    result["syslog"] = lines
if "registers" in layout:
    registers = {}

    def register_node(node, height):
        length = integer(node + layout["register_node_length"], 2)
        if length > 11 or height > 12:
            raise ValueError("VM register tree differs from its measured layout")
        for slot in range(length):
            key = node + layout["register_node_keys"] + 16 * slot
            address, size = integer(key), integer(key + 8)
            if size > 16:
                raise ValueError("VM register name exceeds its measured representation")
            name = inferior.read_memory(address, size).tobytes().decode("ascii")
            registers[name] = integer(node + layout["register_node_values"] + 16 * slot, 16)
        if height:
            for slot in range(length + 1):
                register_node(integer(node + layout["register_node_edges"] + 8 * slot), height - 1)

    tree = machine + layout["registers"]
    register_node(integer(tree), integer(tree + 8))
    result["registers"] = {name: hex(value) for name, value in registers.items()}
    stack = registers.get("rsp", 0)
    words = spec.get("stack_words", 192)
    raw_stack = guest_bytes(stack, words * 8)
    result["stack"] = [hex(int.from_bytes(raw_stack[8 * slot:8 * slot + 8], "little")) for slot in range(words)]
    if "unwind_elf" in spec:
        sys.path.extend(spec.get("python_paths", []))
        from vox_guest_unwind import unwind
        result["frames"] = unwind(spec["unwind_elf"], registers,
                                  int(result["guest_pc"], 16), guest_bytes)
        amplitudes = []
        for frame in result["frames"]:
            if "4mass5visit" not in frame["symbol"] or not 0x547d0 <= int(frame["pc"], 16) < 0x54850:
                continue
            pointer = int(frame["registers"]["r14"], 16)
            words = [guest_integer(pointer + 8 * i) for i in range(8)]
            arena = int(frame["registers"]["r13"], 16)
            arena_words = [guest_integer(arena + 8 * i) for i in range(32)]
            frame_stack = int(frame["registers"]["rsp"], 16)
            node_id = guest_integer(frame_stack + 8)
            node_pointer = arena_words[7] + 64 * node_id
            node_words = [guest_integer(node_pointer + 8 * i) for i in range(8)]
            amplitudes.append({"address": hex(pointer), "words": [hex(value) for value in words],
                               "arena": hex(arena), "arena_words": [hex(value) for value in arena_words],
                               "node_id": node_id, "node_address": hex(node_pointer),
                               "node_words": [hex(value) for value in node_words]})
        result["mass_amplitudes"] = amplitudes
with open(spec["output"], "w") as output:
    json.dump(result, output, indent=2)
    output.write("\n")
print(json.dumps({key: value for key, value in result.items() if key not in ("stack", "frames")}))
