#!/usr/bin/env python3
"""Audit reachable player sprite frames and native source citations.

Checks bank1's 242-entry $4000 table against extracted mappings. Reviewed RAM
value domains and explicit declarations bound index writes; stale declarations,
unknown writers and out-of-range frames fail. Native citations are not behavior
parity; TODO/unported paragraphs do not count. Interpreter details stay beside
the code, CLI/evidence contracts in tools/README.md.
Exit 0: checked; 1: bad/uncited frame; 2: invalid or unresolved source/input."""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from asm_slice import SourceIndex, bank_cpu, instruction_size, rom_address  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent
DECOMP = Path(
    os.environ.get("SML2_DECOMP_ROOT")
    or (ROOT / "decomp" if (ROOT / "decomp").exists() else ROOT.parent / "sml2decomp")
)
ROM_PATH = DECOMP / "baserom.gb"
OBJECTS = ROOT / "extracted" / "objects.json"
ASM_FILES = ("home.asm", "main.asm")
RUST_ROOT = ROOT / "crates"

# A value set larger than this is "unbounded" and must be declared.
MAX_SET = 64
# Predecessor-join depth.  Deep enough for the `$530D`-`$54A7` selector chain.
MAX_DEPTH = 24


@dataclass(frozen=True)
class TableSpec:
    name: str
    hram: int  # the index byte, $FF00 + hram
    ptr_bank: int
    ptr_cpu: int
    count: int
    json_key: str
    # Routines that read the index byte, and what they do with it.  "table"
    # indexes the frame table; anything else reuses the byte for something
    # that is not a frame (its sites are listed but not checked as frames).
    readers: dict[str, str] = field(default_factory=dict)


PLAYER = TableSpec("player", 0xC6, 1, 0x4000, 242, "player_mappings", {
    "unknowncall_0x5297": "table",
    "unknownjump_0x5297": "table",
    # `$5267`/`$5272` draw one tile, `$80 + nibble`, straight into OAM: a
    # digit (stomp chain, save-file tallies), not a table frame.
    "unknowncall_0x5267": "digits",
    "unknowncall_0x5272": "digits",
})
TABLES = {PLAYER.name: PLAYER}

# Value domains of RAM variables the frame selectors read, each reviewed
# against every store to the variable in the disassembly.  A read of any other
# RAM byte is unbounded.
RAM_DOMAINS: dict[str, tuple[frozenset, str]] = {
    "scurpowerup": (frozenset(range(4)), "0 small, 1 big, 2 bunny, 3 fire"),
    "smariodirection": (frozenset([1, 255]), "every store is `ld a,1` or `ld a,255`"),
    "scastlewalkphase": (frozenset(range(3)), "stored after `cp 3 / jr c` resets 3 to 0"),
    "scastleitempreview": (frozenset(range(4)), "copied into sCurPowerup at $606DB"),
    "scastlesavedpowerup": (frozenset(range(4)), "every store copies sCurPowerup ($60917, $60DC7)"),
    "scurlevel": (frozenset(range(32)), "level ids 0..31"),
}
# Leaf routines no longer than this are evaluated in place at a `call`.
LEAF_MAX = 32

# Sites the interpreter cannot bound, keyed by (file, line).  Each value set
# was read off the disassembly by hand; `why` says where it comes from.
# A declaration may also narrow a bounded inferred set (the interpreter does
# not follow `cp`/`jr` guards); the declared set must then be a subset.
DECLARED: dict[str, dict[tuple[str, int], dict]] = {"player": {
    ("main.asm", 16069): {
        "values": [32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 48],
        "why": "$3009B: Easy mode adds 32 to the frame $3012E just chose; $3012E's "
               "file-select selectors ($3012E-$303B7) pick 0-10 and 16",
    },
    ("main.asm", 16709): {
        "values": [0, 196, 215, 216, 217, 218, 219],
        "why": "$3050A indexes the 24-byte $49B5 table by sCurLevel (its sibling "
               "$499D is also 24 bytes); levels >= 24 would read $49CD onward",
    },
    ("main.asm", 37232): {
        "values": [0, 32, 69, 112],
        "why": "$60CDC: $4C4D[e] with e = 0, sCastleSavedPowerup or "
               "sCastleItemPreview, all powerup ids 0..3",
    },
    ("main.asm", 39777): {
        "values": [229, 230, 231],
        "why": "$68477: sMarioWalkAnimTimer wraps at $30, so (t & $F0) >> 4 is 0..2",
    },
    ("main.asm", 39803): {
        "values": [232, 233, 234],
        "why": "$684AB: sMarioWalkAnimTimer wraps at $30, so (t & $F0) >> 4 is 0..2",
    },
}}


# --------------------------------------------------------------------------
# ROM


class Rom:
    def __init__(self, data: bytes):
        self.data = data

    def byte(self, bank: int, cpu: int) -> int | None:
        if cpu < 0x4000:
            off = cpu
        elif cpu < 0x8000:
            off = bank * 0x4000 + (cpu - 0x4000)
        else:
            return None
        return self.data[off] if off < len(self.data) else None

    def word(self, bank: int, cpu: int) -> int:
        return self.byte(bank, cpu) | (self.byte(bank, cpu + 1) << 8)

    def mapping(self, bank: int, cpu: int) -> list[dict]:
        """`$80`-terminated [y, x, tile, attr] cells, as `parse_mapping` reads them."""
        out = []
        while True:
            y = self.byte(bank, cpu)
            if y == 0x80:
                return out
            x, tile, attr = (self.byte(bank, cpu + k) for k in (1, 2, 3))
            out.append({"y": y - 256 if y > 127 else y, "x": x - 256 if x > 127 else x,
                        "tile": tile, "attr": attr})
            cpu += 4


# --------------------------------------------------------------------------
# Assembly


LABEL_RE = re.compile(r"^([A-Za-z_][\w.]*):")
SECTION_RE = re.compile(r'^SECTION\s+"[^"]*",\s*ROMX,\s*BANK\[\$([0-9A-Fa-f]+)\]')
JUMP_RE = re.compile(r"^(jp|jr|call)\s+(?:(?:n?z|n?c)\s*,\s*)?([A-Za-z_][\w.]*)$")
UNCOND_END = re.compile(r"^(jp|jr)\s+[A-Za-z_$(][^,]*$|^(ret|reti)$")
CLOBBER = re.compile(r"^(call|rst)\b")


@dataclass
class Item:
    file: str
    line: int
    bank: int
    kind: str  # "label" | "ins" | "barrier" (INCLUDE/INCBIN: size unknown)
    text: str  # label name or lower-cased, space-normalised instruction
    addr: int | None = None  # CPU address, from `asm_slice.SourceIndex`


@dataclass
class Program:
    items: list[Item]
    labels: dict[str, int]  # name -> index
    rom: bytes = b""
    preds: dict[int, list[int]] = field(default_factory=dict)  # label idx -> jump idxs
    routine_of: list[int] = field(default_factory=list)  # idx -> routine start idx


def norm(code: str) -> str:
    code = re.sub(r"\s+", " ", code.strip())
    code = re.sub(r"\s*,\s*", ", ", code)
    return code.lower()


def parse_program(decomp: Path, rom: bytes) -> Program:
    items: list[Item] = []
    for fname in ASM_FILES:
        bank = 0
        for n, raw in enumerate((decomp / fname).read_text().splitlines(), 1):
            code = raw.split(";", 1)[0].rstrip()
            m = SECTION_RE.match(code.strip())
            if m:
                bank = int(m[1], 16)
                items.append(Item(fname, n, bank, "barrier", code.strip()))
                continue
            m = LABEL_RE.match(code)
            if m:
                items.append(Item(fname, n, bank, "label", m[1]))
                rest = code[m.end():].strip()
                if rest:
                    items.append(Item(fname, n, bank, "ins", norm(rest)))
                continue
            if code.strip().upper().startswith(("INCLUDE", "INCBIN")):
                items.append(Item(fname, n, bank, "barrier", code.strip()))
            elif code.strip():
                items.append(Item(fname, n, bank, "ins", norm(code)))
    labels = {it.text.lower(): i for i, it in enumerate(items) if it.kind == "label"}
    prog = Program(items, labels, rom)
    for i, it in enumerate(items):
        if it.kind != "ins":
            continue
        m = JUMP_RE.match(it.text)
        if m and m[2] in labels:
            prog.preds.setdefault(labels[m[2]], []).append(i)
    assign_addresses(items, SourceIndex.load(decomp, rom))
    # A routine starts at a non-local label (anything but `...RJump_`), or at
    # unlabelled code right after an unconditional `jp`/`jr`/`ret`: the
    # disassembly leaves many jump-table entries without a label.
    cur, ended = 0, False
    for i, it in enumerate(items):
        if it.kind == "label" and "RJump" not in it.text:
            cur = i
        elif it.kind == "ins" and ended:
            cur = i
        ended = it.kind == "ins" and bool(UNCOND_END.match(it.text))
        prog.routine_of.append(cur)
    return prog


def assign_addresses(items: list[Item], index: SourceIndex) -> None:
    """CPU address of every item, from `asm_slice`'s ROM-checked source index.

    A label takes the address of the instruction or data that follows it.
    """
    at = {}
    for rom, where in index.locations.items():
        key = tuple(where.split(" ", 1)[0].split(":"))
        at.setdefault((key[0], int(key[1])), rom)
    pending = []
    for it in items:
        if it.kind == "label":
            pending.append(it)
            continue
        rom = at.get((it.file, it.line))
        it.addr = bank_cpu(rom)[1] if rom is not None else None
        for lab in pending:
            lab.addr = it.addr
        pending = []


def insn_size(rom: bytes, bank: int, cpu: int) -> int:
    return instruction_size(rom[rom_address(bank, cpu)]) if cpu < 0x8000 else 1


def reader_kind(prog: Program, spec: TableSpec, name: str, depth: int = 0) -> str | None:
    """What a call/jump to `name` does with the index byte, if it reaches a reader."""
    if name in spec.readers:
        return spec.readers[name]
    if depth > 4 or name not in prog.labels:
        return None
    k = prog.labels[name] + 1
    while k < len(prog.items) and k - prog.labels[name] < 64:
        it = prog.items[k]
        if it.kind == "ins":
            m = JUMP_RE.match(it.text)
            if m and m[1] in ("call", "jp"):
                r = reader_kind(prog, spec, m[2], depth + 1)
                if r or (m[1] == "jp" and "," not in it.text):
                    return r
            if UNCOND_END.match(it.text) and not it.text.startswith(("jr", "jp")):
                return None
        k += 1
    return None


def site_consumer(prog: Program, spec: TableSpec, i: int, hram_write, hram_read) -> str:
    """Follow straight-line flow from a write to the first reader it reaches.

    Unconditional `jp`/`jr` are followed, conditional ones fall through.
    Returns the reader kind, "overwritten" if another write comes first, or
    "deferred" when the routine returns first (a later draw reads the byte).
    """
    k, steps, branched = i + 1, 0, False
    since_label = True  # no index-byte read seen since the block began
    while k < len(prog.items) and steps < 400:
        steps += 1
        it = prog.items[k]
        if it.kind == "label":
            branched = True  # another path joins here
        if it.kind == "ins":
            if hram_read(it.text):
                since_label = False
            if hram_write(it.text):
                # Only a write that ignores the old byte, on every path from
                # this site, hides the value; `ldh a,[$FFC6] / add n` rewrites
                # and branchy code let it (or its rewrite) through.
                if since_label and not branched:
                    return "overwritten"
                since_label = True
            m = JUMP_RE.match(it.text)
            if m:
                if "," in it.text:
                    branched = True
                r = reader_kind(prog, spec, m[2])
                if r:
                    return r
                if m[1] in ("jp", "jr") and "," not in it.text:
                    if m[2] not in prog.labels:
                        return "deferred"
                    k = prog.labels[m[2]] + 1
                    continue
            elif UNCOND_END.match(it.text):
                return "deferred"
        k += 1
    return "deferred"


# --------------------------------------------------------------------------
# Abstract interpreter
#
# A value is a frozenset of bytes (bounded) or ANY.  `hl` is a frozenset of
# 16-bit pointers or ANY.  The pseudo-register "m" is the table's index byte.

ANY = None
REG8 = ("a", "b", "c", "d", "e", "h", "l", "m")
FULL = frozenset(range(256))


def num(tok: str) -> int | None:
    tok = tok.strip()
    if tok.startswith("$") and re.fullmatch(r"\$[0-9a-f]+", tok):
        return int(tok[1:], 16)
    if tok.startswith("%") and re.fullmatch(r"%[01]+", tok):
        return int(tok[1:], 2)
    if re.fullmatch(r"\d+", tok):
        return int(tok)
    return None


def lift(v):
    return FULL if v is ANY else v


def bound(v):
    return ANY if v is ANY or len(v) > MAX_SET * 4 else v


def amap(v, fn):
    return frozenset(fn(x) & 0xFF for x in lift(v))


def join(states):
    states = [s for s in states if s is not None]
    if not states:
        return None
    out = {}
    for k in set().union(*states):
        vals = [s.get(k, ANY) for s in states]
        out[k] = ANY if any(v is ANY for v in vals) else frozenset().union(*vals)
    return out


class Interp:
    def __init__(self, prog: Program, rom: Rom, spec: TableSpec):
        self.p, self.rom, self.spec = prog, rom, spec
        self.hram = f"[$ff{spec.hram:02x}]"
        self.memo: dict[int, dict | None] = {}

    def is_write(self, text: str) -> bool:
        return text in (f"ldh {self.hram}, a", f"ld {self.hram}, a", f"ldh_n_a ${self.spec.hram:02x}")

    def is_read(self, text: str) -> bool:
        return text in (f"ldh a, {self.hram}", f"ld a, {self.hram}", f"ldh_a_n ${self.spec.hram:02x}")

    # state *before* item i
    def state_before(self, i: int, depth: int = 0, stack: frozenset = frozenset()):
        j = i - 1
        while j >= 0 and self.p.items[j].kind == "ins":
            j -= 1
        # j: the block's label (or -1); instructions j+1 .. i-1 run in order
        if j < 0:
            start = {}
        else:
            start = self.state_at_label(j, depth, stack)
            if start is None:
                return None
        st = dict(start)
        for k in range(j + 1, i):
            st = self.step(st, self.p.items[k])
        return st

    def state_at_label(self, li: int, depth: int, stack: frozenset):
        if self.p.items[li].kind == "barrier":
            return {}
        if li in self.memo:
            return self.memo[li]
        if li in stack:
            return None  # back edge: first iteration only
        if depth >= MAX_DEPTH:
            return {}
        stack = stack | {li}
        ins = []
        prev = li - 1
        while prev >= 0 and self.p.items[prev].kind == "label":
            prev -= 1
        if prev >= 0 and not UNCOND_END.match(self.p.items[prev].text) \
                and self.p.items[prev].file == self.p.items[li].file:
            ins.append(self.after(prev, depth + 1, stack))
        # every label sharing this address (adjacent labels) shares predecessors
        k = li
        while k >= 0 and self.p.items[k].kind == "label":
            for src in self.p.preds.get(k, []):
                ins.append(self.state_before(src, depth + 1, stack))
            k -= 1
        k = li + 1
        while k < len(self.p.items) and self.p.items[k].kind == "label":
            for src in self.p.preds.get(k, []):
                ins.append(self.state_before(src, depth + 1, stack))
            k += 1
        res = join(ins) if ins else {}
        if res is None:
            res = {}
        if depth == 0:
            self.memo[li] = res
        return res

    def after(self, i, depth, stack):
        st = self.state_before(i, depth, stack)
        return None if st is None else self.step(st, self.p.items[i])

    def step(self, st: dict, it: Item) -> dict:
        t = it.text
        if it.kind != "ins":
            return st
        st = dict(st)
        g = lambda r: st.get(r, ANY)
        if self.is_write(t):
            st["m"] = g("a")
            return st
        if self.is_read(t):
            st["a"] = g("m")
            return st
        leaf = self.leaf_body(t)
        if leaf is not None:
            for sub in leaf:
                st = self.step(st, sub)
            return st
        if CLOBBER.match(t):
            m = st.get("m", ANY)
            return {"m": m}  # callee may change any register, and the index byte
        op, _, rest = t.partition(" ")
        args = [x.strip() for x in rest.split(",")] if rest else []
        if op in ("ld", "ldh") and len(args) == 2:
            dst, src = args
            if dst == "hl":
                n = num(src)
                st["hl"] = ANY if n is None else frozenset([n])
                return st
            if dst in ("de", "bc"):
                n = num(src)
                hi, lo = dst
                st[hi] = ANY if n is None else frozenset([n >> 8])
                st[lo] = ANY if n is None else frozenset([n & 0xFF])
                return st
            if dst in REG8:
                n = num(src)
                if n is not None:
                    st[dst] = frozenset([n & 0xFF])
                elif src in REG8:
                    st[dst] = g(src)
                elif src in ("[hl]", "[hli]", "[hld]", "[hl+]", "[hl-]"):
                    st[dst] = self.load_hl(st, it)
                    if src != "[hl]":
                        st["hl"] = ANY if st.get("hl", ANY) is ANY else frozenset(
                            (p + (1 if "i" in src or "+" in src else -1)) & 0xFFFF for p in st["hl"])
                elif src.startswith("[") and src[1:-1] in RAM_DOMAINS:
                    st[dst] = RAM_DOMAINS[src[1:-1]][0]
                else:
                    m = re.fullmatch(r"\[\$([0-9a-f]{4})\]", src)
                    v = self.rom.byte(it.bank, int(m[1], 16)) if m else None
                    st[dst] = frozenset([v]) if v is not None else ANY
                if dst in ("h", "l"):
                    st["hl"] = ANY
                return st
            if dst.startswith("[hl"):
                if dst != "[hl]":
                    st["hl"] = ANY
                return st
            return st
        if op == "ldh_a_n":
            st["a"] = ANY
            return st
        if op == "add" and args[:1] == ["hl"]:
            src = args[1]
            hi, lo = (src[0], src[1]) if src in ("de", "bc") else (None, None)
            base = st.get("hl", ANY)
            if hi and base is not ANY and g(hi) is not ANY and g(lo) is not ANY:
                idx = {(h << 8) | l for h in g(hi) for l in g(lo)}
                st["hl"] = frozenset((b + x) & 0xFFFF for b in base for x in idx)
            else:
                st["hl"] = ANY
            return st
        if op in ("inc", "dec") and args in (["hl"],):
            if st.get("hl", ANY) is not ANY:
                st["hl"] = frozenset((p + (1 if op == "inc" else -1)) & 0xFFFF for p in st["hl"])
            return st
        if op in ("inc", "dec") and args and args[0] in ("bc", "de", "sp"):
            if args[0] != "sp":
                st[args[0][0]] = ANY
                st[args[0][1]] = ANY
            return st
        alu = {"add": lambda x, y: x + y, "sub": lambda x, y: x - y, "and": lambda x, y: x & y,
               "or": lambda x, y: x | y, "xor": lambda x, y: x ^ y}
        if op in alu:
            src = args[-1]
            if op == "xor" and src == "a":
                st["a"] = frozenset([0])
                return st
            n = num(src)
            if n is not None:
                st["a"] = bound(amap(g("a"), lambda x: alu[op](x, n)))
            elif src in REG8:
                a, b = g("a"), g(src)
                if a is ANY and op != "and":
                    st["a"] = ANY
                elif a is ANY or b is ANY:
                    st["a"] = bound(frozenset(alu[op](x, y) & 0xFF for x in lift(a) for y in lift(b)))
                else:
                    st["a"] = bound(frozenset(alu[op](x, y) & 0xFF for x in a for y in b))
            else:
                st["a"] = ANY
            return st
        shifts = {
            "srl": lambda v: v >> 1, "sla": lambda v: v << 1, "sra": lambda v: (v >> 1) | (v & 0x80),
            "swap": lambda v: ((v << 4) | (v >> 4)), "inc": lambda v: v + 1, "dec": lambda v: v - 1,
        }
        if op in shifts and len(args) == 1 and args[0] in REG8:
            r = args[0]
            st[r] = bound(amap(g(r), shifts[op])) if (g(r) is not ANY or op in ("srl", "swap", "sla", "sra")) else ANY
            if g(r) is not ANY and len(g(r)) == 256 and op in ("inc", "dec"):
                st[r] = ANY
            return st
        if op == "cpl":
            st["a"] = ANY if g("a") is ANY else amap(g("a"), lambda v: ~v)
            return st
        if op in ("cp", "bit", "jp", "jr", "nop", "di", "ei", "halt", "scf", "ccf", "push", "ret", "reti"):
            return st
        if op in ("set", "res") and len(args) == 2:
            r = args[1]
            if r in REG8 and g(r) is not ANY:
                b = int(args[0])
                st[r] = amap(g(r), (lambda v: v | (1 << b)) if op == "set" else (lambda v: v & ~(1 << b)))
            return st
        if op == "pop":
            for r in {"af": "a", "bc": "bc", "de": "de", "hl": "hl"}.get(args[0], ""):
                st[r] = ANY
            if args[0] == "hl":
                st["hl"] = ANY
            return st
        if op in ("ldh", "ld"):
            return st
        # anything else that names a register as its destination clobbers it
        if args and args[0] in REG8:
            st[args[0]] = ANY
        if args and args[0] == "hl":
            st["hl"] = ANY
        if op in ("rla", "rra", "rlca", "rrca", "daa", "adc", "sbc", "rl", "rr", "rlc", "rrc"):
            st["a"] = ANY if not args or args[0] in ("a",) or op in ("adc", "sbc") else st.get("a", ANY)
        return st

    def leaf_body(self, text: str):
        """A `call`ed routine that runs straight to `ret`: its instructions."""
        m = JUMP_RE.match(text)
        if not m or m[1] != "call" or "," in text or m[2] not in self.p.labels:
            return None
        body = []
        k = self.p.labels[m[2]] + 1
        while k < len(self.p.items) and len(body) < LEAF_MAX:
            sub = self.p.items[k]
            if sub.kind == "ins":
                if sub.text == "ret":
                    return body
                if sub.text.startswith(("jp", "jr", "call", "ret", "rst")):
                    return None
                body.append(sub)
            k += 1
        return None

    def load_hl(self, st, it):
        hl = st.get("hl", ANY)
        if hl is ANY or len(hl) > MAX_SET * 4:
            return ANY
        out = set()
        for p in hl:
            v = self.rom.byte(it.bank, p)
            if v is None:
                return ANY
            out.add(v)
        return frozenset(out)


# --------------------------------------------------------------------------
# Rust citations


CITE_RE = re.compile(r"(?:\$|0x)([0-9A-Fa-f]{4,5})\b")


DISCLAIM_RE = re.compile(r"not (?:modell?ed|ported|implemented)|unported|\bTODO\b", re.I)


def comment_blocks(text: str):
    """Runs of consecutive comment lines, then each code line on its own.

    A citation belongs to the paragraph it is written in, so a disclaimer
    anywhere in that paragraph voids it.
    """
    block: list[str] = []
    for line in text.splitlines():
        t = line.strip()
        if t.startswith("//"):
            block.append(t)
            continue
        if block:
            yield "\n".join(block)
            block = []
        yield t
    if block:
        yield "\n".join(block)


def rust_citations(root: Path) -> tuple[dict[int, list[str]], dict[str, set[int]]]:
    """Address -> non-test source files that mention it as `$XXXX`/`0xXXXX`,
    and file -> the switchable banks (2+) it cites by ROM offset."""
    out: dict[int, set[str]] = {}
    banks: dict[str, set[int]] = {}
    for p in sorted(root.rglob("*.rs")):
        rel = p.relative_to(ROOT).as_posix()
        if "/target/" in rel or "/tests/" in rel or p.name in ("tests.rs",) or p.stem.endswith("_tests"):
            continue
        for block in comment_blocks(p.read_text(errors="replace")):
            if DISCLAIM_RE.search(block):
                continue  # "Not modelled: ... `$30884` ..." is not a port
            for m in CITE_RE.finditer(block):
                a = int(m[1], 16)
                out.setdefault(a, set()).add(rel)
                if a >= 0x8000:
                    banks.setdefault(rel, set()).add(a >> 14)
    return {k: sorted(v) for k, v in out.items()}, banks


def cited_range(bank: int, lo: int, hi: int, anchors: set[int], cites) -> dict[str, list[str]]:
    """`{"$addr": [files]}` for native citations of the routine [lo, hi).

    An unambiguous address counts anywhere in the routine: a home-bank CPU
    address, or a ROM offset of banked code (`$3005F`).  A `$4000-$7FFF` CPU
    address is shared by every switchable bank, so it counts only at the
    routine's entry and label `anchors`; for banks 2+ it also has to come
    from a file that cites that bank by ROM offset.
    """
    by_addr, file_banks = cites
    out = {}
    for a in range(lo, hi):
        if a < 0x4000:
            if a in by_addr:
                out[f"${a:04X}"] = by_addr[a]
            continue
        if bank >= 2:
            rom = bank * 0x4000 + (a - 0x4000)
            if rom in by_addr:
                out[f"${rom:04X}"] = by_addr[rom]
        if a not in anchors or a not in by_addr:
            continue
        fs = by_addr[a] if bank < 2 else [f for f in by_addr[a] if bank in file_banks.get(f, ())]
        if fs:
            out[f"${a:04X}"] = fs
    return out


def routine_name(prog: Program, i: int) -> str:
    it = prog.items[prog.routine_of[i]]
    if it.kind == "label":
        return it.text
    return f"(unlabelled ${it.addr:04X})" if it.addr is not None else "(unlabelled)"


def enclosing_code(prog: Program, i: int) -> int | None:
    """For a site in an unlabelled or `Data`-labelled region: the nearest
    earlier code label (`Call`/`Jump`/named) in the same file and bank."""
    r = prog.routine_of[i]
    start = prog.items[r]
    if start.kind == "label" and "Data" not in start.text:
        return None
    k = r - 1
    while k >= 0:
        it = prog.items[k]
        if it.kind == "barrier" or it.file != start.file:
            return None
        if it.kind == "label" and "RJump" not in it.text and "Data" not in it.text:
            return k + 1 if k + 1 < len(prog.items) else None
        k -= 1
    return None


def routine_range(prog: Program, i: int) -> tuple[int, int, set[int]] | None:
    """[start, end) CPU addresses of item i's routine, and its label addresses."""
    r = prog.routine_of[i]
    lo = hi = None
    anchors = set()
    k = r
    while k > 0 and prog.items[k - 1].kind == "label":
        k -= 1  # adjacent aliases of the entry
    while k < len(prog.items) and (k <= r or prog.routine_of[k] == r):
        it = prog.items[k]
        if it.addr is not None:
            lo = it.addr if lo is None else lo
            if it.kind == "label" or k == r:
                anchors.add(it.addr)
            n = insn_size(prog.rom, it.bank, it.addr) if it.kind == "ins" else 0
            hi = it.addr + n
        k += 1
    return None if lo is None else (lo, max(hi, lo + 1), anchors)


# --------------------------------------------------------------------------
# Inventory


def fmt(vals) -> str:
    if vals is ANY:
        return "ANY"
    v = sorted(vals)
    return "{" + ", ".join(str(x) for x in v) + "}" if len(v) <= 12 else f"{{{v[0]}..{v[-1]}}} ({len(v)})"


def inventory(spec: TableSpec, prog: Program, rom: Rom, objects: dict, cites: dict,
              declared: dict) -> tuple[dict, list[str], list[str]]:
    interp = Interp(prog, rom, spec)
    errors: list[str] = []  # exit 2
    fails: list[str] = []  # exit 1
    sites = []
    seen_declared = set()
    for i, it in enumerate(prog.items):
        if it.kind != "ins" or not interp.is_write(it.text):
            continue
        st = interp.state_before(i)
        vals = ANY if st is None else st.get("a", ANY)
        key = (it.file, it.line)
        decl = declared.get(key)
        consumer = (decl or {}).get("consumer") or site_consumer(prog, spec, i, interp.is_write, interp.is_read)
        source = "inferred"
        if decl is not None:
            seen_declared.add(key)
        if decl is not None and "values" in decl:
            narrowed = frozenset(decl["values"])
            if vals is not ANY and not narrowed <= vals:
                errors.append(f"{it.file}:{it.line}: DECLARED values {fmt(narrowed)} are not a subset "
                              f"of the inferred {fmt(vals)}")
            vals, source = narrowed, "declared"
        if vals is ANY or len(vals) > MAX_SET:
            if consumer not in ("table", "deferred"):
                vals = frozenset()  # not a frame: nothing to bound
            else:
                errors.append(f"{it.file}:{it.line}: unresolved {spec.name} write ({fmt(vals)}) "
                              "is not in DECLARED")
                vals = frozenset()
                source = "unresolved"
        rng = routine_range(prog, i)
        cited_by = cited_range(it.bank, *rng, cites) if rng else {}
        via = "routine" if cited_by else None
        outer = enclosing_code(prog, i)
        if not cited_by and outer is not None:
            # An unlabelled jump-table entry (or code the disassembly labels
            # `Data`) is usually cited through the dispatcher that owns it.
            # Span the dispatcher, its `rst $28` table and the entries up to
            # this one: comments cite any of them.
            a, b = routine_range(prog, outer), routine_range(prog, i)
            cited_by = cited_range(it.bank, a[0], b[1], a[2] | b[2], cites) if a and b else {}
            via = "enclosing" if cited_by else None
        sites.append({
            "file": it.file, "line": it.line, "bank": it.bank,
            "routine": routine_name(prog, i),
            "values": sorted(vals), "source": source, "consumer": consumer,
            "why": declared.get(key, {}).get("why"),
            "cited": sorted(cited_by), "cited_via": via, "cited_files": sorted({f for v in cited_by.values() for f in v}),
        })
    for key in set(declared) - seen_declared:
        errors.append(f"{key[0]}:{key[1]}: DECLARED {spec.name} site is not a write site (stale)")

    # frames
    ptrs = [rom.word(spec.ptr_bank, spec.ptr_cpu + 2 * n) for n in range(spec.count)]
    extracted = objects.get(spec.json_key, [])
    if len(extracted) != spec.count:
        fails.append(f"extracted {spec.json_key} has {len(extracted)} entries, ROM table {spec.count}")
    frames = []
    by_frame: dict[int, list[dict]] = {}
    for s in sites:
        if s["consumer"] not in ("table", "deferred"):
            continue
        for v in s["values"]:
            by_frame.setdefault(v, []).append(s)
    for v in sorted(by_frame):
        if v >= spec.count:
            where = ", ".join(f"{s['file']}:{s['line']}" for s in by_frame[v])
            fails.append(f"frame {v} is past the {spec.count}-entry table (from {where})")
    first_of_ptr: dict[int, int] = {}
    for n, ptr in enumerate(ptrs):
        first_of_ptr.setdefault(ptr, n)
        cells = rom.mapping(spec.ptr_bank, ptr)
        data_ok = n < len(extracted) and extracted[n] == cells
        sel = by_frame.get(n, [])
        if not sel:
            status = "unreachable"
        elif not data_ok:
            status = "data-mismatch"
        elif all(s["cited"] for s in sel):
            status = "cited"
        else:
            status = "uncited"
        if sel and not data_ok:
            fails.append(f"frame {n}: extracted mapping differs from ROM ${ptr:04X}")
        frames.append({
            "frame": n, "ptr": f"${ptr:04X}", "alias_of": first_of_ptr[ptr] if first_of_ptr[ptr] != n else None,
            "cells": len(cells), "data_ok": data_ok, "status": status,
            # `*` marks a selecting site no native source cites
            "sites": [f"{s['file']}:{s['line']}{'' if s['cited'] else '*'}" for s in sel],
        })
    for s in sites:
        if s["values"] and not s["cited"]:
            fails.append(f"{s['file']}:{s['line']} ({s['routine']}) selects {fmt(frozenset(s['values']))} "
                         "and no native source cites its routine")
    return {"table": spec.name, "sites": sites, "frames": frames}, errors, fails


def report(inv: dict, show_frames: bool, quiet: bool) -> None:
    frames, sites = inv["frames"], inv["sites"]
    counts: dict[str, int] = {}
    for f in frames:
        counts[f["status"]] = counts.get(f["status"], 0) + 1
    if not quiet:
        print(f"== {inv['table']} frames: {len(frames)} table entries, {len(sites)} write sites "
              f"({sum(s['source'] == 'declared' for s in sites)} declared)")
        print("   " + ", ".join(f"{k} {v}" for k, v in sorted(counts.items())))
    if show_frames and not quiet:
        print(f"   {'frame':>5} {'ptr':>6} {'cells':>5}  {'status':<13} sites (* = uncited)")
        for f in frames:
            alias = f" (= {f['alias_of']})" if f["alias_of"] is not None else ""
            print(f"   {f['frame']:>5} {f['ptr']:>6} {f['cells']:>5}  {f['status']:<13} "
                  f"{', '.join(f['sites'])}{alias}")


# --------------------------------------------------------------------------


def fail(msg: str) -> None:
    print(f"anim_inventory: {msg}", file=sys.stderr)
    raise SystemExit(2)


def load():
    for p in (ROM_PATH, OBJECTS, *(DECOMP / f for f in ASM_FILES)):
        if not p.is_file():
            fail(f"missing input {p}")
    data = ROM_PATH.read_bytes()
    return Rom(data), json.loads(OBJECTS.read_text()), parse_program(DECOMP, data)


def run(rom, objects, prog, cites, declared_all):
    out, errors, fails = {}, [], []
    for name, spec in TABLES.items():
        inv, e, f = inventory(spec, prog, rom, objects, cites, declared_all.get(name, {}))
        out[name] = inv
        errors += e
        fails += f
    return out, errors, fails


def self_test(rom, objects, prog, cites) -> int:
    ok = True
    # 1. an undeclared unresolved site is an input error
    if DECLARED["player"]:
        k = next(iter(DECLARED["player"]))
        trimmed = {"player": {kk: v for kk, v in DECLARED["player"].items() if kk != k}}
        _, errors, _ = run(rom, objects, prog, cites, trimmed)
        ok &= any(f"{k[0]}:{k[1]}" in e for e in errors)
    # 2. a stale declaration is an input error
    stale = {"player": {**DECLARED["player"], ("main.asm", 1): {"values": [0], "why": "test"}}}
    _, errors, _ = run(rom, objects, prog, cites, stale)
    ok &= any("main.asm:1:" in e and "stale" in e for e in errors)
    # 3. a corrupted extracted mapping fails its frame
    bad = json.loads(json.dumps(objects))
    bad["player_mappings"][0] = []
    _, _, fails = run(rom, bad, prog, cites, DECLARED)
    ok &= any(f.startswith("frame 0:") for f in fails)
    # 4. without its `cp $30` narrowing the credits walk runs off the table
    loose = {"player": {k: v for k, v in DECLARED["player"].items() if k != ("main.asm", 39803)}}
    _, _, fails = run(rom, objects, prog, cites, loose)
    ok &= any("past the 242-entry table" in f and "main.asm:39803" in f for f in fails)
    # 5. dropping every citation fails every site
    _, _, fails = run(rom, objects, prog, ({}, {}), DECLARED)
    ok &= any("no native source cites" in f for f in fails)
    print("anim_inventory self-test:", "ok" if ok else "FAILED")
    return 0 if ok else 1


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--json", type=Path)
    ap.add_argument("--frames", action="store_true", help="print the per-frame checklist")
    ap.add_argument("--quiet", action="store_true", help="only failures")
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()

    rom, objects, prog = load()
    cites = rust_citations(RUST_ROOT)
    if args.self_test:
        return self_test(rom, objects, prog, cites)
    out, errors, fails = run(rom, objects, prog, cites, DECLARED)
    for inv in out.values():
        report(inv, args.frames, args.quiet)
    if args.json:
        args.json.write_text(json.dumps(out, indent=1) + "\n")
    for e in errors:
        print(f"ERROR {e}")
    for f in fails:
        print(f"FAIL  {f}")
    if errors:
        return 2
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
