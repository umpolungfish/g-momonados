#!/usr/bin/env python3
"""Rebuild a binary's control-flow wiring as Diagrammar and executable IMASM.

The input is Vox's ``vox-wiring-v1`` TSV (or an ELF plus the Vox executable)
and one Diagrammar ASCII circuit. The output keeps every instruction address,
adds explicit FSPLIT edges for machine branches, places FFUSE witnesses at
nearest common postdominators, and records call/return re-entry edges. The
writer refuses to mark the whole image closed when an indirect jump, uncovered
code range, or branch without a real reconvergence remains unresolved.

The rewritten IMASM tape carries the complete instruction image, explicit
FFUSE marks at address-backed reconvergences, and preserved virtual outcomes.
Inserted FFUSE records are execution-neutral in Vox's IMASM machine.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import re
import subprocess
import sys
from collections import defaultdict, deque
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable


WORK_GLYPHS = {"≻", "≺", "⋈", "⊤", "⊥", "⊞", "⊡"}
NODE_RE = re.compile(r"^\s*(\d+)\s+(\S+)\s+(\S+)")
EDGE_RE = re.compile(r"^\s*[├└]─▶\s*(\d+)\s+(\S+)\s+\[([^]]+)\]")
HEADER_RE = re.compile(r"(\d+)\s+nodes,\s+(\d+)\s+wires\s+open forks\s+(\d+)")
CIRCUIT_RE = re.compile(r"◇\s+CIRCUIT\s+(\S+)")


@dataclass(frozen=True)
class Instruction:
    function: int
    address: int
    length: int
    glyph: str
    mnemonic: str
    operands: str
    target: int | None
    fallthrough: int | None


def _hex(value: str) -> int | None:
    value = value.strip()
    return int(value, 16) if value else None


def read_vox_wiring_text(text: str, source: str = "<memory>") -> tuple[dict, list[Instruction]]:
    lines = text.splitlines()
    if not lines or not lines[0].startswith("# vox-wiring-v1\t"):
        raise ValueError(f"{source} is not Vox vox-wiring-v1 output")
    head = lines[0].split("\t")
    metadata = {"entry": int(head[1], 16), "claimed_bytes": int(head[2]), "total_bytes": int(head[3]),
                "symbols": [], "function_ranges": [], "executable_ranges": [], "opaque_bytes": []}
    out: list[Instruction] = []
    table = False
    for line in lines[1:]:
        if line.startswith("# symbol\t"):
            _, address, name = line.split("\t", 2)
            metadata["symbols"].append({"address": int(address, 16), "name": name})
            continue
        if line.startswith("# function-range\t"):
            _, start, end = line.split("\t", 2)
            metadata["function_ranges"].append((int(start, 16), int(end, 16)))
            continue
        if line.startswith("# executable-range\t"):
            _, start, end = line.split("\t", 2)
            metadata["executable_ranges"].append((int(start, 16), int(end, 16)))
            continue
        if line.startswith("# opaque-byte\t"):
            _, address, byte = line.split("\t", 2)
            metadata["opaque_bytes"].append((int(address, 16), int(byte, 16)))
            continue
        if line.startswith("function\t"):
            table = True
            continue
        if not table or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) != 8:
            raise ValueError(f"malformed Vox wiring row: {line[:180]}")
        function, address, length, glyph, mnemonic, operands, target, fallthrough = fields
        out.append(Instruction(int(function, 16), int(address, 16), int(length, 16), glyph,
                               mnemonic, operands, _hex(target), _hex(fallthrough)))
    if not out:
        raise ValueError("Vox wiring contained no decoded instructions")
    return metadata, out


def read_vox_wiring(path: Path) -> tuple[dict, list[Instruction]]:
    return read_vox_wiring_text(path.read_text(errors="replace"), str(path))


def read_diagrammar(path: Path) -> dict:
    return read_diagrammar_text(path.read_text(errors="replace"), str(path))


def read_diagrammar_text(text: str, source: str = "<memory>") -> dict:
    first = text.splitlines()[0] if text.splitlines() else ""
    match = HEADER_RE.search(first)
    circuit_match = CIRCUIT_RE.search(first)
    if not match:
        raise ValueError(f"{source} has no recognized Diagrammar circuit header")
    declared_nodes, declared_edges, open_forks = map(int, match.groups())
    nodes: dict[int, dict] = {}
    edges: list[dict] = []
    active: int | None = None
    for line in text.splitlines():
        n = NODE_RE.match(line)
        if n:
            active = int(n.group(1))
            nodes[active] = {"glyph": n.group(2), "operation": n.group(3)}
            continue
        e = EDGE_RE.match(line)
        if e and active is not None:
            edges.append({"from": active, "to": int(e.group(1)), "glyph": e.group(2), "ports": e.group(3)})
    walk_closed = any(re.match(r"^\s*walk:\s+CLOSED\b", line) for line in text.splitlines())
    return {"header": first, "circuit": circuit_match.group(1) if circuit_match else None,
            "declared_nodes": declared_nodes, "declared_edges": declared_edges,
            "declared_open_forks": open_forks, "parsed_nodes": len(nodes), "parsed_edges": len(edges),
            "declared_walk_closed": walk_closed, "nodes": nodes, "edges": edges,
            "format_consistent": len(nodes) == declared_nodes and len(edges) == declared_edges}


def read_dispatch_map(path: Path | None) -> dict[int, list[int]]:
    if path is None:
        return {}
    data = json.loads(path.read_text())
    # Accept a single prior-probe record or a map keyed by indirect-jump PC.
    if isinstance(data, list):
        records = data
    elif "dispatch_instruction" in data:
        records = [data]
    else:
        records = data.get("dispatches", [])
    result: dict[int, list[int]] = {}
    for record in records:
        pc = int(record["dispatch_instruction"], 0)
        result[pc] = [int(t, 0) for t in record.get("targets", [])]
    return result


def _in_executable_ranges(address: int | None, ranges: list[tuple[int, int]]) -> bool:
    return address is not None and any(start <= address < end for start, end in ranges)


def _successors(ins: Instruction, by_addr: dict[int, Instruction], by_address_global: dict[int, Instruction],
                dispatch: dict[int, list[int]], executable_ranges: list[tuple[int, int]]) -> tuple[list[int], list[dict]]:
    mn = ins.mnemonic.lower()
    edges: list[int] = []
    links: list[dict] = []
    if mn.startswith("ret") or mn in {"ud2", "hlt", "int3", "iret", "sysret"}:
        return edges, links
    if mn == "call":
        if ins.target is None:
            links.append({"kind": "indirect_call_frame", "site": ins.address,
                          "operand": ins.operands, "target_set": "runtime expression preserved",
                          "return_to": ins.fallthrough})
            if ins.fallthrough in by_address_global:
                edges.append(ins.fallthrough)
        else:
            target = by_address_global.get(ins.target)
            links.append({"kind": "call_frame", "site": ins.address, "target": ins.target,
                          "target_function": target.function if target else None, "return_to": ins.fallthrough})
            if ins.fallthrough in by_address_global:
                # The intraprocedural CFG uses the call summary. The separate
                # call-frame relation records entry and exact re-entry so a
                # shared callee is never returned to another caller's frame.
                edges.append(ins.fallthrough)
            else:
                edges.append(-ins.address - 2)
        return edges, links
    if mn == "jmp":
        if ins.target is not None:
            if ins.target in by_addr:
                edges.append(ins.target)
            elif ins.target in by_address_global:
                links.append({"kind": "cross_function_tail_transfer", "site": ins.address,
                              "target": ins.target, "target_function": by_address_global[ins.target].function})
                edges.append(ins.target)
            else:
                kind = "tail_transfer_into_undecoded_image" if _in_executable_ranges(ins.target, executable_ranges) else "external_tail_transfer"
                links.append({"kind": kind, "site": ins.address, "target": ins.target})
                edges.append(-ins.address - 2)
        elif ins.address in dispatch:
            for target in dispatch[ins.address]:
                if target in by_addr:
                    edges.append(target)
                elif target in by_address_global:
                    links.append({"kind": "cross_function_dispatch_target", "site": ins.address,
                                  "target": target, "target_function": by_address_global[target].function})
                    edges.append(target)
                else:
                    kind = "dispatch_target_into_undecoded_image" if _in_executable_ranges(target, executable_ranges) else "external_dispatch_target"
                    links.append({"kind": kind, "site": ins.address, "target": target})
                    edges.append(-ins.address - 2)
        else:
            links.append({"kind": "dynamic_indirect_tail_transfer", "site": ins.address,
                          "operand": ins.operands, "target_set": "runtime expression preserved"})
            edges.append(-ins.address - 2)
        return list(dict.fromkeys(edges)), links
    if mn.startswith("j"):
        if ins.target is not None and ins.target in by_addr:
            edges.append(ins.target)
        elif ins.target is not None and ins.target in by_address_global:
            links.append({"kind": "cross_function_conditional_branch", "site": ins.address,
                          "target": ins.target, "target_function": by_address_global[ins.target].function})
            edges.append(ins.target)
        else:
            kind = "conditional_target_into_undecoded_image" if _in_executable_ranges(ins.target, executable_ranges) else "external_conditional_target"
            links.append({"kind": kind, "site": ins.address,
                          "target": ins.target})
            edges.append(-ins.address - 2)
        if ins.fallthrough in by_address_global:
            edges.append(ins.fallthrough)
        else:
            kind = "conditional_fallthrough_into_undecoded_image" if _in_executable_ranges(ins.fallthrough, executable_ranges) else "executable_section_boundary"
            links.append({"kind": kind, "site": ins.address,
                          "target": ins.fallthrough})
            edges.append(-ins.address - 2)
        return list(dict.fromkeys(edges)), links
    if ins.fallthrough in by_address_global:
        edges.append(ins.fallthrough)
    else:
        edges.append(-ins.address - 2)
        kind = "fallthrough_into_undecoded_image" if _in_executable_ranges(ins.fallthrough, executable_ranges) else "executable_section_boundary"
        links.append({"kind": kind, "site": ins.address,
                      "target": ins.fallthrough})
    return edges, links


def _strong_components(nodes: set[int], successors: dict[int, list[int]],
                       reverse: dict[int, list[int]] | None = None) -> dict[int, int]:
    if reverse is None:
        reverse = defaultdict(list)
        for src, targets in successors.items():
            for dst in targets:
                if dst in nodes:
                    reverse[dst].append(src)
    seen: set[int] = set()
    finish: list[int] = []
    for start in nodes:
        if start in seen:
            continue
        seen.add(start)
        walk: list[tuple[int, object]] = [(start, iter(successors.get(start, ())))]
        while walk:
            node, children_iter = walk[-1]
            try:
                child = next(children_iter)
            except StopIteration:
                finish.append(node)
                walk.pop()
                continue
            if child in nodes and child not in seen:
                seen.add(child)
                walk.append((child, iter(successors.get(child, ()))))
    component: dict[int, int] = {}
    component_id = 0
    for start in reversed(finish):
        if start in component:
            continue
        component_id += 1
        component[start] = component_id
        todo = [start]
        while todo:
            node = todo.pop()
            for pred in reverse.get(node, ()):
                if pred in nodes and pred not in component:
                    component[pred] = component_id
                    todo.append(pred)
    return component


def _postdominator_tree(nodes: set[int], successors: dict[int, list[int]], exits: set[int]) -> tuple[dict[int, int], dict[int, int], dict[int, int]]:
    """Build the postdominator tree with Lengauer-Tarjan on the reversed CFG.

    This is linear in the CFG size in practice, unlike storing a set of
    postdominators for every instruction. Nodes that cannot reach any exit
    remain absent, so a branch into a nonterminating component cannot be
    falsely certified as closed.
    """
    root = -1
    succ = {n: [x for x in successors.get(n, ()) if x in nodes] for n in nodes}
    terminals = {n for n in nodes if n in exits or not succ[n]}
    reverse: dict[int, list[int]] = defaultdict(list)
    for src, targets in succ.items():
        for dst in targets:
            reverse[dst].append(src)
    reverse[root] = sorted(terminals)

    component = _strong_components(nodes, succ, reverse)

    # Strong components expose explicit loop re-entry points. A conditional
    # arm in the same SCC as its split can return to that split even when it
    # has no terminating path, which ordinary postdominance intentionally
    # leaves open.
    #
    # (The graph builder can add tagged divergence outcomes before this pass;
    # this component map remains useful for reporting loop re-entry witnesses.)
    # DFS the reversed CFG from the synthetic exit. Its outgoing edges are
    # original predecessors; original successors are LT predecessor lists.
    dfs_number = {root: 1}
    vertex = [root]
    dfs_parent: dict[int, int] = {}
    stack: list[tuple[int, object]] = [(root, iter(reverse[root]))]
    while stack:
        node, children = stack[-1]
        try:
            child = next(children)
        except StopIteration:
            stack.pop()
            continue
        if child not in dfs_number:
            dfs_number[child] = len(vertex) + 1
            vertex.append(child)
            dfs_parent[child] = node
            stack.append((child, iter(reverse.get(child, ()))))

    semi = dict(dfs_number)
    label = {node: node for node in vertex}
    ancestor: dict[int, int | None] = {node: None for node in vertex}
    bucket: dict[int, list[int]] = defaultdict(list)
    idom: dict[int, int] = {root: root}

    def evaluate(node: int) -> int:
        if ancestor[node] is None:
            return label[node]
        path: list[int] = []
        cur = node
        while ancestor[cur] is not None and ancestor[ancestor[cur]] is not None:
            path.append(cur)
            cur = ancestor[cur]  # type: ignore[assignment]
        for item in reversed(path):
            parent = ancestor[item]
            if parent is not None and semi[label[parent]] < semi[label[item]]:
                label[item] = label[parent]
            if parent is not None:
                ancestor[item] = ancestor[parent]
        return label[node]

    for node in reversed(vertex[1:]):
        # Predecessors in the reversed CFG are successors in the original.
        preds = list(succ.get(node, ()))
        if node in terminals:
            preds.append(root)
        for pred in preds:
            if pred not in dfs_number:
                continue
            candidate = evaluate(pred)
            semi[node] = min(semi[node], semi[candidate])
        semi_vertex = vertex[semi[node] - 1]
        bucket[semi_vertex].append(node)
        parent = dfs_parent[node]
        ancestor[node] = parent
        for pending in bucket[parent]:
            candidate = evaluate(pending)
            idom[pending] = candidate if semi[candidate] < semi[pending] else parent
        bucket[parent].clear()

    for node in vertex[1:]:
        semi_vertex = vertex[semi[node] - 1]
        if idom[node] != semi_vertex:
            idom[node] = idom[idom[node]]

    depth = {root: 0}
    children: dict[int, list[int]] = defaultdict(list)
    for node, parent in idom.items():
        if node != root:
            children[parent].append(node)
    todo = [root]
    while todo:
        parent = todo.pop()
        for child in children[parent]:
            depth[child] = depth[parent] + 1
            todo.append(child)
    return idom, depth, component


def _nearest_real_common_postdom(arms: list[int], parent: dict[int, int], depth: dict[int, int]) -> int | None:
    if len(arms) < 2 or any(arm not in parent for arm in arms):
        return None
    left, right = arms[0], arms[1]
    while depth[left] > depth[right]:
        left = parent[left]
    while depth[right] > depth[left]:
        right = parent[right]
    while left != right:
        left, right = parent[left], parent[right]
    return left


def _reachable_until(start: int, stop: int, successors: dict[int, list[int]], by_addr: dict[int, Instruction]) -> set[int]:
    seen: set[int] = set()
    todo = [start]
    while todo:
        cur = todo.pop()
        if cur == stop or cur in seen or cur not in by_addr:
            continue
        seen.add(cur)
        todo.extend(successors.get(cur, []))
    return seen


def rewire(metadata: dict, instructions: list[Instruction], dispatch: dict[int, list[int]], diagram: dict) -> dict:
    by_function: dict[int, list[Instruction]] = defaultdict(list)
    by_address_global = {i.address: i for i in instructions}
    for ins in instructions:
        by_function[ins.function].append(ins)
    all_edges: list[dict] = []
    all_nodes: list[dict] = []
    all_links: list[dict] = []
    all_pairs: list[dict] = []
    unresolved: list[dict] = []
    succ: dict[int, list[int]] = {}
    call_returns: dict[int, list[tuple[int, int]]] = defaultdict(list)
    returns_by_function: dict[int, list[int]] = defaultdict(list)

    # Build the executable CFG over the whole image. Calls are intraprocedural
    # summaries here; exact nested entry and return relationships are preserved
    # as frame records instead of context-insensitive cross-caller CFG edges.
    for function, insns in sorted(by_function.items()):
        insns.sort(key=lambda i: i.address)
        by_addr = {i.address: i for i in insns}
        local_links: list[dict] = []
        for ins in insns:
            out, links = _successors(ins, by_addr, by_address_global, dispatch,
                                     metadata.get("executable_ranges", []))
            succ[ins.address] = out
            local_links.extend(links)
            if ins.mnemonic.lower() == "call" and ins.target in by_address_global and ins.fallthrough is not None:
                callee = by_address_global[ins.target].function
                call_returns[callee].append((ins.address, ins.fallthrough))
                all_edges.append({"function": function, "from": ins.address, "to": ins.target,
                                  "kind": "call-entry-frame", "frame_return_to": ins.fallthrough})
            if ins.mnemonic.lower().startswith("ret"):
                returns_by_function[function].append(ins.address)
            for target in out:
                all_edges.append({"function": function, "from": ins.address, "to": target,
                                  "kind": "branch-target" if ins.mnemonic.startswith("j") and ins.mnemonic != "jmp" and target == ins.target
                                  else "fallthrough" if target == ins.fallthrough else
                                  "synthetic-transfer" if target < -1 else "call-or-dispatch"})
            all_nodes.append({"id": f"0x{ins.address:x}", "address": ins.address, "function": function,
                              "glyph": ins.glyph, "mnemonic": ins.mnemonic, "operands": ins.operands})
        all_links.extend({"function": function, **link} for link in local_links)
        unresolved.extend({"function": function, **link} for link in local_links if link["kind"] in {
            "tail_transfer_into_undecoded_image", "dispatch_target_into_undecoded_image",
            "conditional_target_into_undecoded_image", "conditional_fallthrough_into_undecoded_image",
            "fallthrough_into_undecoded_image"})

    # Bind every callee return to its exact caller frames. These are stack
    # re-entry relations, not ordinary edges: adding every return-to-every
    # caller edge would invent impossible paths for shared callees.
    for function, sites in call_returns.items():
        for ret in returns_by_function.get(function, []):
            all_links.append({"function": function, "kind": "return_reentry", "site": ret,
                              "frames": [{"call_site": site, "callee_function": function,
                                          "return_instruction": ret, "return_to": resume}
                                         for site, resume in sites]})

    # Materialize symbolic transfer exits as graph nodes. Their outgoing target
    # expression remains in the transfer-frame record and is not guessed.
    synthetic_nodes = sorted({target for targets in succ.values() for target in targets if target < -1})
    for node in synthetic_nodes:
        all_nodes.append({"id": f"xfer:{-node - 2:x}", "site": -node - 2,
                          "kind": "dynamic-or-outside-image-transfer", "glyph": "⊙"})
        succ[node] = []

    base_addresses = set(succ)
    base_components = _strong_components(base_addresses, succ)
    component_nodes: dict[int, list[int]] = defaultdict(list)
    for address, component_id in base_components.items():
        component_nodes[component_id].append(address)
    divergence_sccs: list[dict] = []
    closure_succ = {address: targets.copy() for address, targets in succ.items()}
    next_virtual = min([-1, *closure_succ.keys()]) - 1
    for component_id, members in component_nodes.items():
        member_set = set(members)
        has_real_exit = any(not succ[address] for address in members)
        has_outgoing = any(target not in member_set for address in members for target in succ[address])
        if has_real_exit or has_outgoing:
            continue
        outcome = next_virtual
        next_virtual -= 1
        closure_succ[outcome] = []
        for address in members:
            closure_succ[address].append(outcome)
        divergence_sccs.append({"outcome_node": f"diverge:{component_id}", "outcome_address": outcome,
                                "component": component_id,
                                "node_count": len(members), "address_min": min(members),
                                "address_max": max(members), "tag": "infinite-trace"})
        all_nodes.append({"id": f"diverge:{component_id}", "site": outcome,
                          "kind": "divergence-outcome",
                          "glyph": "⊙", "tag": "infinite-trace"})

    addresses = set(closure_succ)
    exits = {address for address, targets in closure_succ.items() if not targets}
    post_parent, post_depth, components = _postdominator_tree(addresses, closure_succ, exits)
    for function, insns in sorted(by_function.items()):
        for ins in insns:
            if not ins.mnemonic.lower().startswith("j") or ins.mnemonic.lower() == "jmp":
                continue
            arms = succ.get(ins.address, [])
            join = _nearest_real_common_postdom(arms, post_parent, post_depth)
            if join is None:
                loop_arms = [arm for arm in arms if components.get(arm) == components.get(ins.address)]
                terminating_arms = [arm for arm in arms if arm not in loop_arms]
                if loop_arms and all(arm in post_parent for arm in terminating_arms):
                    all_pairs.append({"function": function, "split": ins.address, "arms": arms,
                                      "fuse": {"kind": "loop-reentry", "address": ins.address,
                                               "reentering_arms": loop_arms},
                                      "status": "CLOSED", "closure": "SCC re-entry at split"})
                    continue
                all_pairs.append({"function": function, "split": ins.address, "arms": arms,
                                  "fuse": None, "status": "OPEN", "reason": "an arm has no terminating re-entry path"})
                continue
            if join == -1:
                fuse = {"kind": "whole-image-outcome", "preserves_terminal_tags": True}
            elif join < -1 and join in succ:
                fuse = {"kind": "symbolic-transfer-outcome", "site": -join - 2}
            else:
                divergence = next((item for item in divergence_sccs
                                   if item["outcome_address"] == join), None)
                fuse = ({"kind": "divergence-outcome", "component": divergence["component"]}
                        if divergence else join)
            all_pairs.append({"function": function, "split": ins.address, "arms": arms,
                              "fuse": fuse, "status": "CLOSED", "closure": "control-flow postdominator"})
    open_pairs = [p for p in all_pairs if p["status"] != "CLOSED"]
    for pair in all_pairs:
        pair["arm_entry_work_witness"] = any(
            by_address_global.get(arm) is not None
            and by_address_global[arm].glyph in WORK_GLYPHS
            for arm in pair["arms"])
    opaque_addresses = {address for address, _ in metadata.get("opaque_bytes", [])}
    accounted_bytes = metadata["claimed_bytes"] + len(opaque_addresses)
    coverage_ok = accounted_bytes == metadata["total_bytes"]
    circuit = diagram.get("circuit")
    diagram_symbols = [symbol for symbol in metadata.get("symbols", [])
                       if circuit and circuit in symbol["name"]]
    diagrammar_closed = (diagram["format_consistent"] and diagram["declared_open_forks"] == 0
                         and diagram.get("declared_walk_closed", False)
                         and bool(diagram_symbols))
    dynamic_frames = [link for link in all_links if link["kind"] in {
        "dynamic_indirect_tail_transfer", "indirect_call_frame"}]
    symbolic_transfers_complete = all(link.get("operand") for link in dynamic_frames)
    transformation_witness = any(pair["arm_entry_work_witness"] for pair in all_pairs)
    closure_ok = (not open_pairs and not unresolved and not opaque_addresses and diagrammar_closed
                  and symbolic_transfers_complete and transformation_witness)
    # Diagrammar is a structural reference; it is retained verbatim as evidence
    # while the address-labelled graph is rebuilt from Vox edges.
    return {
        "format": "vox-diagrammar-rewired-v1",
        "source": {"entry": metadata["entry"], "claimed_bytes": metadata["claimed_bytes"],
                   "opaque_byte_count": len(opaque_addresses), "accounted_bytes": accounted_bytes,
                   "total_bytes": metadata["total_bytes"], "instruction_count": len(instructions),
                   "function_count": len(by_function)},
        "diagrammar_input": diagram,
        "diagrammar_anchor": {"circuit": circuit,
                              "symbols": [{"address": s["address"], "name": s["name"]} for s in diagram_symbols]},
        "rewiring": {"method": "whole-image address CFG with call summaries, exact nested re-entry frames, and dynamic transfer frames",
                     "nodes": all_nodes, "edges": all_edges, "call_and_transfer_frames": all_links,
                     "frobenius_pairs": all_pairs,
                     "divergence_outcomes": divergence_sccs,
                     "preserved_opaque_bytes": [[f"0x{a:x}", f"0x{b:02x}"] for a, b in metadata.get("opaque_bytes", [])]},
        "checks": {"whole_image_coverage": coverage_ok, "whole_image_decoded": not opaque_addresses,
                   "diagrammar_format": diagram["format_consistent"],
                   "diagrammar_fork_closure": diagram["declared_open_forks"] == 0,
                   "diagrammar_walk_closure": diagram.get("declared_walk_closed", False),
                   "diagrammar_symbol_anchor": bool(diagram_symbols),
                   "frobenius_closure": closure_ok, "closed_pairs": sum(p["status"] == "CLOSED" for p in all_pairs),
                   "arm_entry_work_witness_pairs": sum(p["arm_entry_work_witness"] for p in all_pairs),
                   "arm_entries_without_work_witness": sum(not p["arm_entry_work_witness"] for p in all_pairs),
                   "frobenius_transformation_witness": transformation_witness,
                   "open_pairs": sum(p["status"] == "OPEN" for p in all_pairs),
                   "unresolved_transfers": len(unresolved),
                   "symbolic_transfers_preserved": symbolic_transfers_complete,
                   "symbolic_transfer_count": len(dynamic_frames),
                   "frobenius_scope": "interprocedural control-flow closure; value-level Frobenius identity requires a semantic witness",
                   "publishable": coverage_ok and closure_ok},
        "unresolved": unresolved,
    }


def _vox_text(elf: Path, vox: Path) -> str:
    # Use the real Vox decoder as the source of address-level instructions.
    result = subprocess.run([str(vox), "disasm", str(elf), "--wiring-complete"], check=True,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    return result.stdout


def _vox_imasm(elf: Path, vox: Path, output: Path) -> None:
    result = subprocess.run([str(vox), "imasm", str(elf), "--output", str(output)],
                            check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if not output.is_file() or output.stat().st_size == 0:
        raise ValueError(f"Vox produced no executable IMASM module for {elf}: {result.stderr}")


def _write_output(path: Path, result: dict) -> None:
    opener = gzip.open if path.suffix == ".gz" else open
    with opener(path, "wt", encoding="utf-8") as output:
        for chunk in json.JSONEncoder(indent=2).iterencode(result):
            output.write(chunk)
        output.write("\n")


def _diagrammar_text(result: dict) -> str:
    """Render the complete rebuilt graph in the existing Diagrammar ASCII form."""
    nodes = result["rewiring"]["nodes"]
    edges = result["rewiring"]["edges"]
    pairs = result["rewiring"]["frobenius_pairs"]
    node_id = {node["id"]: index for index, node in enumerate(nodes)}
    address_id = {}
    for node in nodes:
        if "address" in node:
            address_id[node["address"]] = node_id[node["id"]]
        if "site" in node:
            address_id[node["site"]] = node_id[node["id"]]
            address_id[-node["site"] - 2] = node_id[node["id"]]
    outgoing: dict[int, list[tuple[int, str]]] = defaultdict(list)
    for edge in edges:
        src = address_id.get(edge["from"])
        dst = address_id.get(edge["to"])
        if src is not None and dst is not None:
            outgoing[src].append((dst, edge["kind"]))
    split_sites = {pair["split"]: pair for pair in pairs}
    rendered_edge_count = sum(map(len, outgoing.values()))
    lines = [f"◇ CIRCUIT vox_rewired {len(nodes)} nodes, {rendered_edge_count} wires open forks {sum(p['status'] != 'CLOSED' for p in pairs)}",
             "  address-level executable wiring; edge labels retain Vox transfer kind"]
    for index, node in enumerate(nodes):
        address = node.get("address")
        pair = split_sites.get(address)
        glyph = "∈" if pair is not None else node.get("glyph", "⊙")
        if address is not None:
            operation = f"{node.get('mnemonic', 'INSTRUCTION')}@0x{address:x}"
        else:
            operation = node.get("kind", "NODE").replace(" ", "_")
        lines.append(f"{index:7d} {glyph} {operation}")
        for target, kind in outgoing.get(index, []):
            lines.append(f"     └─▶ {target:7d} {glyph} [o→i:{kind}]")
    for pair in pairs:
        fuse = pair.get("fuse")
        if fuse is None:
            fuse_text = "OPEN"
        elif isinstance(fuse, int) and fuse >= 0:
            fuse_text = f"0x{fuse:x}"
        elif isinstance(fuse, int):
            fuse_text = f"symbolic-transfer@0x{-fuse - 2:x}"
        elif fuse.get("kind") == "symbolic-transfer-outcome":
            fuse_text = f"symbolic-transfer@0x{fuse['site']:x}"
        else:
            fuse_text = str(fuse.get("kind", "fuse"))
        arms = ",".join(f"0x{arm:x}" for arm in pair["arms"])
        lines.append(f"# frobenius-pair split=0x{pair['split']:x} arms={arms} fuse={fuse_text} status={pair['status']}")
    lines.append("\n  walk: " + ("CLOSED" if result["checks"]["frobenius_closure"] else "OPEN"))
    return "\n".join(lines) + "\n"


def _write_diagrammar(path: Path, result: dict) -> None:
    opener = gzip.open if path.suffix == ".gz" else open
    with opener(path, "wt", encoding="utf-8") as output:
        output.write(_diagrammar_text(result))


def _imasm_addresses(module: str) -> tuple[set[int], dict[int, set[str]]]:
    addresses: set[int] = set()
    glyphs: dict[int, set[str]] = defaultdict(set)
    current: int | None = None
    for line in module.splitlines():
        if line.startswith("@0x"):
            current = int(line[3:], 16)
            addresses.add(current)
        elif current is not None and line and not line.startswith((";", "=")):
            glyphs[current].add(line.split("\t", 1)[0][:1])
    return addresses, glyphs


def _rewrite_imasm_text(module: str, result: dict) -> tuple[str, dict]:
    """Insert executable no-op FFUSE marks into Vox's complete IMASM tape."""
    nodes = result["rewiring"]["nodes"]
    expected_addresses = {node["address"] for node in nodes if "address" in node}
    source_addresses, source_glyphs = _imasm_addresses(module)
    split_sites = {pair["split"] for pair in result["rewiring"]["frobenius_pairs"]}
    missing_splits = sorted(address for address in split_sites
                            if address not in source_addresses or "∈" not in source_glyphs.get(address, set()))
    coverage_exact = source_addresses == expected_addresses

    fuses_at: dict[int, list[dict]] = defaultdict(list)
    virtual_pairs: list[dict] = []
    for pair in result["rewiring"]["frobenius_pairs"]:
        fuse = pair["fuse"]
        if isinstance(fuse, int) and fuse >= 0:
            fuses_at[fuse].append(pair)
        elif isinstance(fuse, dict) and fuse.get("kind") == "loop-reentry":
            fuses_at[fuse["address"]].append(pair)
        else:
            virtual_pairs.append(pair)
    missing_fuses = sorted(address for address in fuses_at if address not in source_addresses)

    lines = module.splitlines()
    header = 1 if lines and lines[0] == "; standalone static-v1" else 0
    if len(lines) <= header or not lines[header].startswith("; ⊙ module ("):
        raise ValueError("Vox did not emit a complete executable IMASM module")
    rewritten: list[str] = [*lines[:header + 1],
        "; @vox-binary-rewire:v1",
        f"; @vox-binary-rewire:source-sha256 {hashlib.sha256(module.encode()).hexdigest()}",
        f"; @vox-binary-rewire:frobenius CLOSED pairs={len(result['rewiring']['frobenius_pairs'])}",
        f"; @vox-binary-rewire:instruction-addresses {len(source_addresses)}"]
    rewritten.extend(lines[header + 1:])
    out: list[str] = []
    inserted = 0
    for line in rewritten:
        out.append(line)
        if line.startswith("@0x"):
            address = int(line[3:], 16)
            for pair in fuses_at.get(address, []):
                out.append(f"; @vox-binary-rewire:fuse split=0x{pair['split']:x} at=0x{address:x}")
                out.append("∋")
                inserted += 1
    out.extend([f"; @vox-binary-rewire:pair split=0x{pair['split']:x} "
                f"arms={','.join(f'0x{arm:x}' for arm in pair['arms'])} "
                f"fuse={pair['fuse'].get('kind') if isinstance(pair['fuse'], dict) else pair['fuse']} "
                f"status={pair['status']}"
                for pair in virtual_pairs])
    reentry_records = [link for link in result["rewiring"]["call_and_transfer_frames"]
                       if link["kind"] == "return_reentry"]
    reentry_frames = sum(len(link["frames"]) for link in reentry_records)
    for link in reentry_records:
        for frame in link["frames"]:
            out.append(f"; @vox-binary-rewire:reentry ret=0x{frame['return_instruction']:x} "
                       f"call=0x{frame['call_site']:x} return=0x{frame['return_to']:x}")
    dynamic_records = [link for link in result["rewiring"]["call_and_transfer_frames"]
                       if link["kind"] in {"dynamic_indirect_tail_transfer", "indirect_call_frame"}]
    for link in dynamic_records:
        out.append(f"; @vox-binary-rewire:dynamic-transfer kind={link['kind']} "
                   f"site=0x{link['site']:x} operand={link['operand']} "
                   f"return=0x{link['return_to']:x}" if link["kind"] == "indirect_call_frame" else
                   f"; @vox-binary-rewire:dynamic-transfer kind={link['kind']} "
                   f"site=0x{link['site']:x} operand={link['operand']}")
    tape = "\n".join(out) + "\n"
    tape_addresses, tape_glyphs = _imasm_addresses(tape)
    comment_pairs = sum(line.startswith("; @vox-binary-rewire:pair ") for line in out)
    reentry_comments = sum(line.startswith("; @vox-binary-rewire:reentry ") for line in out)
    dynamic_comments = sum(line.startswith("; @vox-binary-rewire:dynamic-transfer ") for line in out)
    # An inline fuse is distinguishable from source code by its adjacent marker.
    normalized = []
    pending_fuse = False
    for line in tape.splitlines():
        if line.startswith("; @vox-binary-rewire:"):
            pending_fuse = line.startswith("; @vox-binary-rewire:fuse ")
            continue
        if pending_fuse and line == "∋":
            pending_fuse = False
            continue
        pending_fuse = False
        normalized.append(line)
    source_digest = hashlib.sha256(("\n".join(module.splitlines()) + "\n").encode()).hexdigest()
    normalized_digest = hashlib.sha256(("\n".join(normalized) + "\n").encode()).hexdigest()
    check = {
        "source_address_coverage": coverage_exact,
        "all_branch_splits_present": not missing_splits,
        "all_fuse_addresses_present": not missing_fuses,
        "inline_fuse_count": inserted,
        "expected_inline_fuse_count": sum(map(len, fuses_at.values())),
        "virtual_outcome_pairs": len(virtual_pairs),
        "pair_records_present": comment_pairs == len(virtual_pairs),
        "nested_reentry_frames": reentry_frames,
        "nested_reentry_records_present": reentry_comments == reentry_frames,
        "dynamic_transfer_records": len(dynamic_records),
        "dynamic_transfer_records_present": dynamic_comments == len(dynamic_records),
        "instruction_stream_preserved": source_digest == normalized_digest,
        "rewritten_address_coverage": tape_addresses == expected_addresses,
        "fuse_marker_glyph": all("∋" in tape_glyphs.get(address, set())
                                  for address in fuses_at),
        "missing_splits": [f"0x{address:x}" for address in missing_splits[:32]],
        "missing_fuses": [f"0x{address:x}" for address in missing_fuses[:32]],
        "publishable": (coverage_exact and not missing_splits and not missing_fuses
                        and inserted == sum(map(len, fuses_at.values()))
                        and comment_pairs == len(virtual_pairs)
                        and reentry_comments == reentry_frames
                        and dynamic_comments == len(dynamic_records)
                        and source_digest == normalized_digest
                        and tape_addresses == expected_addresses
                        and all("∋" in tape_glyphs.get(address, set()) for address in fuses_at)),
    }
    return tape, check


def _write_imasm(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--vox-wiring", type=Path, help="Vox vox-wiring-v1 TSV")
    parser.add_argument("--elf", type=Path, help="ELF to disassemble with --vox")
    parser.add_argument("--vox", type=Path, help="Vox executable, required with --elf")
    parser.add_argument("--diagrammar", type=Path, required=True, help="Diagrammar ASCII circuit output")
    parser.add_argument("--dispatch-map", type=Path, help="resolved indirect jump target sets as JSON")
    parser.add_argument("--output", type=Path, required=True, help="rewired JSON output")
    parser.add_argument("--diagrammar-output", type=Path,
                        help="complete rewritten Diagrammar ASCII output (default: alongside JSON, gzip when .gz)")
    parser.add_argument("--imasm-output", type=Path,
                        help="rewritten complete executable IMASM tape (requires --elf and --vox)")
    args = parser.parse_args(argv)
    try:
        if args.vox_wiring:
            metadata, insns = read_vox_wiring(args.vox_wiring)
        elif args.elf and args.vox:
            raw = _vox_text(args.elf, args.vox)
            metadata, insns = read_vox_wiring_text(raw, f"Vox output for {args.elf}")
        else:
            parser.error("provide --vox-wiring or both --elf and --vox")
        diagram = read_diagrammar(args.diagrammar)
        result = rewire(metadata, insns, read_dispatch_map(args.dispatch_map), diagram)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        diagrammar_output = args.diagrammar_output or args.output.with_suffix(".diagrammar.txt.gz")
        diagrammar_output.parent.mkdir(parents=True, exist_ok=True)
        default_imasm_name = args.output.name
        for suffix in (".json.gz", ".json"):
            if default_imasm_name.endswith(suffix):
                default_imasm_name = default_imasm_name[:-len(suffix)]
                break
        imasm_output = args.imasm_output or args.output.with_name(default_imasm_name + ".imasm")
        result["checks"]["rewritten_imasm_tape"] = False
        result["imasm_tape"] = {"path": str(imasm_output), "checks": {"publishable": False}}
        if not (args.elf and args.vox):
            result["checks"]["publishable"] = False
            result["imasm_tape"]["reason"] = "a complete ELF and Vox emitter are required to produce the rewritten executable tape"
        elif result["checks"]["publishable"]:
            source_output = imasm_output.with_name(imasm_output.stem + ".source.imasm")
            _vox_imasm(args.elf, args.vox, source_output)
            source_module = source_output.read_text(errors="strict")
            rewritten_module, tape_checks = _rewrite_imasm_text(source_module, result)
            result["imasm_tape"]["source"] = str(source_output)
            result["imasm_tape"]["checks"] = tape_checks
            result["checks"]["rewritten_imasm_tape"] = tape_checks["publishable"]
            result["checks"]["publishable"] = result["checks"]["publishable"] and tape_checks["publishable"]
            if tape_checks["publishable"]:
                _write_imasm(imasm_output, rewritten_module)
        if result["checks"]["rewritten_imasm_tape"]:
            result["checks"]["rewritten_diagrammar"] = False
            diagrammar_text = _diagrammar_text(result)
            roundtrip = read_diagrammar_text(diagrammar_text, "generated Diagrammar")
            diagrammar_ok = (roundtrip["format_consistent"] and roundtrip["declared_walk_closed"]
                             == result["checks"]["frobenius_closure"])
            result["checks"]["rewritten_diagrammar"] = diagrammar_ok
            result["checks"]["publishable"] = result["checks"]["publishable"] and diagrammar_ok
            _write_diagrammar(diagrammar_output, result)
        else:
            result["checks"]["rewritten_diagrammar"] = False
            _write_diagrammar(diagrammar_output, result)
        result["checks"]["publishable"] = result["checks"]["publishable"] and result["checks"]["rewritten_diagrammar"]
        _write_output(args.output, result)
        checks = result["checks"]
        print(f"rewired {result['source']['instruction_count']} instructions in {result['source']['function_count']} functions")
        print(f"Frobenius wiring: {checks['closed_pairs']} closed, {checks['open_pairs']} open; arm-entry work witness: {checks['arm_entry_work_witness_pairs']}/{checks['closed_pairs']}")
        print(f"coverage: {result['source']['accounted_bytes']}/{result['source']['total_bytes']} bytes; opaque bytes: {result['source']['opaque_byte_count']}; unresolved transfers: {checks['unresolved_transfers']}; symbolic transfers preserved: {checks['symbolic_transfer_count']}")
        print(f"Diagrammar: {diagrammar_output}")
        print(f"IMASM tape: {imasm_output if checks['rewritten_imasm_tape'] else 'not emitted'}")
        print(f"result: {'PASS' if checks['publishable'] else 'INCOMPLETE'} ({args.output})")
        return 0 if checks["publishable"] else 2
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"vox_binary_rewire: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
