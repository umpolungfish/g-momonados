#!/usr/bin/env python3
"""Behavior tests for whole-image wiring closure and Diagrammar emission."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("vox_binary_rewire.py")
SPEC = importlib.util.spec_from_file_location("vox_binary_rewire_under_test", MODULE_PATH)
rewirer = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = rewirer
SPEC.loader.exec_module(rewirer)


def fixture_diagram(open_forks: int = 0) -> dict:
    return rewirer.read_diagrammar_text(
        "◇ CIRCUIT apply_nested 1 nodes, 0 wires open forks " + str(open_forks)
        + "\n  0 ⊙ IMSCRIB\n\n  walk: " + ("OPEN" if open_forks else "CLOSED") + "\n"
    )


def fixture_program() -> tuple[dict, list]:
    metadata = {
        "entry": 0x100,
        "claimed_bytes": 4,
        "total_bytes": 4,
        "symbols": [{"address": 0x100, "name": "apply_nested"}],
        "function_ranges": [(0x100, 0x104)],
        "executable_ranges": [(0x100, 0x104)],
        "opaque_bytes": [],
    }
    instructions = [
        rewirer.Instruction(0x100, 0x100, 1, "∈", "je", "0x102", 0x102, 0x101),
        rewirer.Instruction(0x100, 0x101, 1, "⋈", "mov", "rax, rbx", None, 0x103),
        rewirer.Instruction(0x100, 0x102, 1, "⊤", "add", "rax, 1", None, 0x103),
        rewirer.Instruction(0x100, 0x103, 1, "⊣", "ret", "", None, None),
    ]
    return metadata, instructions


class RewireTests(unittest.TestCase):
    def test_diamond_gets_postdominating_fuse_and_complete_diagrammar(self) -> None:
        metadata, instructions = fixture_program()
        result = rewirer.rewire(metadata, instructions, {}, fixture_diagram())
        pair = result["rewiring"]["frobenius_pairs"][0]
        self.assertEqual(pair["status"], "CLOSED")
        self.assertEqual(pair["fuse"], 0x103)
        parsed = rewirer.read_diagrammar_text(rewirer._diagrammar_text(result))
        self.assertTrue(parsed["format_consistent"])
        self.assertEqual(parsed["parsed_nodes"], len(result["rewiring"]["nodes"]))
        self.assertEqual(parsed["parsed_edges"], len(result["rewiring"]["edges"]))
        self.assertTrue(parsed["declared_walk_closed"])
        self.assertTrue(result["checks"]["publishable"])
        self.assertEqual(result["checks"]["arm_entry_work_witness_pairs"], 1)

    def test_diagrammar_open_fork_blocks_publishable_result(self) -> None:
        metadata, instructions = fixture_program()
        result = rewirer.rewire(metadata, instructions, {}, fixture_diagram(open_forks=1))
        self.assertFalse(result["checks"]["diagrammar_fork_closure"])
        self.assertFalse(result["checks"]["publishable"])

    def test_frobenius_requires_transformed_arm_witness(self) -> None:
        metadata, instructions = fixture_program()
        instructions[1] = rewirer.Instruction(0x100, 0x101, 1, "⊙", "nop", "", None, 0x103)
        instructions[2] = rewirer.Instruction(0x100, 0x102, 1, "⊙", "nop", "", None, 0x103)
        result = rewirer.rewire(metadata, instructions, {}, fixture_diagram())
        self.assertFalse(result["checks"]["frobenius_transformation_witness"])
        self.assertFalse(result["checks"]["frobenius_closure"])
        self.assertFalse(result["checks"]["publishable"])

    def test_opaque_executable_byte_blocks_publishable_result(self) -> None:
        metadata, instructions = fixture_program()
        metadata["claimed_bytes"] = 3
        metadata["opaque_bytes"] = [(0x104, 0x90)]
        metadata["total_bytes"] = 4
        result = rewirer.rewire(metadata, instructions, {}, fixture_diagram())
        self.assertFalse(result["checks"]["whole_image_decoded"])
        self.assertFalse(result["checks"]["publishable"])

    def test_unresolved_fallthrough_is_counted_as_an_unresolved_transfer(self) -> None:
        metadata, instructions = fixture_program()
        metadata["executable_ranges"] = [(0x100, 0x106)]
        instructions[1] = rewirer.Instruction(0x100, 0x101, 1, "⋈", "mov", "rax, rbx", None, 0x105)
        result = rewirer.rewire(metadata, instructions, {}, fixture_diagram())
        self.assertEqual(result["checks"]["unresolved_transfers"], 1)
        self.assertEqual(result["unresolved"][0]["kind"], "fallthrough_into_undecoded_image")

    def test_shared_callee_returns_only_through_recorded_call_frames(self) -> None:
        metadata = {
            "entry": 0x100,
            "claimed_bytes": 5,
            "total_bytes": 5,
            "symbols": [{"address": 0x100, "name": "apply_nested"}],
            "function_ranges": [(0x100, 0x102), (0x110, 0x112), (0x200, 0x201)],
            "executable_ranges": [(0x100, 0x201)],
            "opaque_bytes": [],
        }
        instructions = [
            rewirer.Instruction(0x100, 0x100, 1, "⊙", "call", "0x200", 0x200, 0x101),
            rewirer.Instruction(0x100, 0x101, 1, "⊣", "ret", "", None, None),
            rewirer.Instruction(0x110, 0x110, 1, "⊙", "call", "0x200", 0x200, 0x111),
            rewirer.Instruction(0x110, 0x111, 1, "⊣", "ret", "", None, None),
            rewirer.Instruction(0x200, 0x200, 1, "⊣", "ret", "", None, None),
        ]
        result = rewirer.rewire(metadata, instructions, {}, fixture_diagram())
        frames = [link for link in result["rewiring"]["call_and_transfer_frames"]
                  if link["kind"] == "return_reentry" and link["site"] == 0x200]
        self.assertEqual(len(frames), 1)
        self.assertEqual(
            {(frame["call_site"], frame["return_to"]) for frame in frames[0]["frames"]},
            {(0x100, 0x101), (0x110, 0x111)},
        )
        self.assertFalse(any(edge["kind"] == "return-reentry"
                             for edge in result["rewiring"]["edges"]))

    def test_rewritten_imasm_tape_inserts_fuse_and_preserves_instruction_stream(self) -> None:
        metadata, instructions = fixture_program()
        result = rewirer.rewire(metadata, instructions, {}, fixture_diagram())
        result["rewiring"]["call_and_transfer_frames"].append({
            "kind": "indirect_call_frame", "site": 0x101,
            "operand": "r:rax", "target_set": "runtime expression preserved",
            "return_to": 0x102,
        })
        source = (
            "; ⊙ module (elf x86-64)\n; entry 0x100\n; bits 64\n"
            "@0x100\n∈\tje\ti:0x102\n"
            "@0x101\n⋈\tmov\tr:rax\tr:rbx\n"
            "@0x102\n⊤\tadd\tr:rax\ti:0x1\n"
            "@0x103\n⊣\tret\n"
        )
        tape, check = rewirer._rewrite_imasm_text(source, result)
        self.assertTrue(check["publishable"], check)
        self.assertTrue(check["instruction_stream_preserved"])
        self.assertEqual(check["inline_fuse_count"], 1)
        self.assertTrue(check["dynamic_transfer_records_present"])
        self.assertIn("@0x103\n; @vox-binary-rewire:fuse split=0x100 at=0x103\n∋\n⊣\tret", tape)

    def test_imasm_tape_refuses_a_missing_branch_split(self) -> None:
        metadata, instructions = fixture_program()
        result = rewirer.rewire(metadata, instructions, {}, fixture_diagram())
        source = "; ⊙ module (elf x86-64)\n; entry 0x100\n@0x103\n⊣\tret\n"
        _, check = rewirer._rewrite_imasm_text(source, result)
        self.assertFalse(check["all_branch_splits_present"])
        self.assertFalse(check["publishable"])

    def test_static_module_header_and_instruction_payload_survive_rewiring(self) -> None:
        metadata, instructions = fixture_program()
        result = rewirer.rewire(metadata, instructions, {}, fixture_diagram())
        source = (
            "; standalone static-v1\n; ⊙ module (elf x86-64)\n; entry 0x100\n; bits 64\n"
            "@0x100\n∈\tje\ti:0x102\n@0x101\n⋈\tmov\tr:rax\tr:rbx\n"
            "@0x102\n⊤\tadd\tr:rax\ti:0x1\n@0x103\n⊣\tret\n"
        )
        tape, check = rewirer._rewrite_imasm_text(source, result)
        self.assertTrue(check["publishable"], check)
        self.assertTrue(check["instruction_stream_preserved"])
        self.assertTrue(tape.startswith("; standalone static-v1\n; ⊙ module (elf x86-64)\n"))

    def test_static_header_does_not_admit_a_missing_module_header(self) -> None:
        metadata, instructions = fixture_program()
        result = rewirer.rewire(metadata, instructions, {}, fixture_diagram())
        with self.assertRaisesRegex(ValueError, "complete executable IMASM"):
            rewirer._rewrite_imasm_text("; standalone static-v1\n@0x100\n⊣\tret\n", result)

    def test_symbolic_transfer_fuse_stays_a_virtual_tape_relation(self) -> None:
        metadata, instructions = fixture_program()
        result = rewirer.rewire(metadata, instructions, {}, fixture_diagram())
        result["rewiring"]["frobenius_pairs"][0]["fuse"] = {
            "kind": "symbolic-transfer-outcome", "site": 0x100
        }
        tape, check = rewirer._rewrite_imasm_text(
            "; ⊙ module (elf x86-64)\n; entry 0x100\n@0x100\n∈\tje\ti:0x102\n"
            "@0x101\n⋈\tmov\tr:rax\tr:rbx\n@0x102\n⊤\tadd\tr:rax\ti:0x1\n"
            "@0x103\n⊣\tret\n", result)
        self.assertTrue(check["publishable"], check)
        self.assertEqual(check["inline_fuse_count"], 0)
        self.assertEqual(check["virtual_outcome_pairs"], 1)
        self.assertIn("fuse=symbolic-transfer-outcome", tape)


if __name__ == "__main__":
    unittest.main()
