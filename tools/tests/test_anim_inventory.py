"""anim_inventory contracts on synthetic inputs; no ROM or disassembly needed."""
from __future__ import annotations

from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import anim_inventory as inv

SELECTOR = """\
SECTION "bank01", ROMX, BANK[$01]

UnknownCall_0x4000:
\tld a, [sCurPowerup]
\tand $01
\tld e, a
\tld d, 0
\tld hl, $4100
\tadd hl, de
\tld a, [hl]
\tldh [$FFC6], a
\tld a, [sStarTimer]
\tand a
\tjr z, UnknownRJump_0x4020
\tldh a, [$FFC6]
\tadd 32
\tldh [$FFC6], a

UnknownRJump_0x4020:
\tcall UnknownCall_0x5297
\tret
\tld a, [sStompChainCount]
\tldh [$FFC6], a
\tcall UnknownCall_0x5272
\tret

UnknownCall_0x5297:
\tret

UnknownCall_0x5272:
\tret
"""


class InterpreterTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        (root / "home.asm").write_text("")
        (root / "main.asm").write_text(SELECTOR)
        rom = bytearray(0x8000)  # all `nop`: sizes are irrelevant here
        rom[0x4100:0x4102] = b"\x07\x09"
        self.rom = bytes(rom)
        self.prog = inv.parse_program(root, self.rom)
        self.interp = inv.Interp(self.prog, inv.Rom(self.rom), inv.PLAYER)

    def site(self, n):
        writes = [i for i, it in enumerate(self.prog.items)
                  if it.kind == "ins" and self.interp.is_write(it.text)]
        i = writes[n]
        st = self.interp.state_before(i)
        consumer = inv.site_consumer(self.prog, inv.PLAYER, i, self.interp.is_write, self.interp.is_read)
        return st.get("a", inv.ANY), consumer

    def test_table_lookup_reads_rom_through_a_ram_domain(self):
        self.assertEqual(self.site(0), (frozenset({7, 9}), "table"))

    def test_rewrite_sees_the_value_that_reached_it(self):
        self.assertEqual(self.site(1), (frozenset({39, 41}), "table"))

    def test_digit_reader_is_not_a_frame(self):
        vals, consumer = self.site(2)
        self.assertIs(vals, inv.ANY)
        self.assertEqual(consumer, "digits")

    def test_unlabelled_code_after_ret_starts_a_routine(self):
        i = next(i for i, it in enumerate(self.prog.items) if it.text == "ld a, [sstompchaincount]")
        self.assertEqual(inv.routine_name(self.prog, i)[:12], "(unlabelled ")


class CitationTests(unittest.TestCase):
    def test_banked_cpu_address_needs_a_file_citing_that_bank(self):
        cites = ({0x4042: ["a.rs", "b.rs"], 0x30042: ["c.rs"]}, {"a.rs": {12}})
        got = inv.cited_range(12, 0x4040, 0x4050, {0x4042}, cites)
        self.assertEqual(got, {"$4042": ["a.rs"], "$30042": ["c.rs"]})

    def test_ambiguous_cpu_address_counts_only_at_an_anchor(self):
        cites = ({0x5299: ["player.rs"]}, {})
        self.assertEqual(inv.cited_range(1, 0x5297, 0x52A0, {0x5297}, cites), {})
        self.assertEqual(inv.cited_range(1, 0x5297, 0x52A0, {0x5299}, cites), {"$5299": ["player.rs"]})

    def test_home_bank_address_counts_anywhere_in_the_routine(self):
        cites = ({0x28AB: ["game_over.rs"]}, {})
        self.assertEqual(inv.cited_range(0, 0x28A7, 0x28C0, set(), cites), {"$28AB": ["game_over.rs"]})

    def test_disclaimed_paragraphs_and_tests_cite_nothing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            src = root / "crates" / "core" / "src"
            (src / "tests").mkdir(parents=True)
            (src / "a.rs").write_text(
                "/// `$3005F`: the menu loop.\nfn a() {}\n"
                "/// Not modelled: the `$30884` icons.\n/// (see `$30885`)\nfn b() {}\n")
            (src / "tests" / "t.rs").write_text("// `$5297`\n")
            with patch.object(inv, "ROOT", root):
                by_addr, banks = inv.rust_citations(root / "crates")
        self.assertEqual(by_addr, {0x3005F: ["crates/core/src/a.rs"]})
        self.assertEqual(banks, {"crates/core/src/a.rs": {12}})


if __name__ == "__main__":
    unittest.main()
