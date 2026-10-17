# Tooling ticket: checked ending interrupt entry contracts

Implement the bounded plan below. Owned files ONLY tools/asm_slice.py,
tools/handler_task.py, tools/tests/test_porting_tools.py and tools/README.md
(contract command documentation). A small tools ending-contract manifest/module
is allowed if needed to keep the slicer focused. No native/runtime/queue edits.
Read PORTING/native guide and named files; bounded source witness ranges below.
This is source-extraction tooling, not a ready native port of INCOMPLETE slices.
No fake helper binding. Existing generic INCOMPLETE behavior must remain intact.

Deliver checked source profiles, exact evidence output and regression/mutation
checks described below. Generate scoped packets in /tmp for root review; do not
edit game source/queue/inventory/captures or commit. Python tests independent
of audio Rust fullsuite lane; may run them immediately. Report implementation
scope, failed mutations, checks and exact scoped packet paths.

# Bounded ending interrupt slicer prerequisite

Read-only audit; no repository edits, test executions, mutations, captures, or commits. Read PORTING.md/native guide previously in this worker. Inspected tools/asm_slice.py, tools/handler_task.py, tools/tests/test_porting_tools.py, /tmp/sml2-ending-audit/plan.md, and bounded ROM/source excerpts. Used SourceIndex only to annotate literal ROM bytes; did not run slices/tests.

## Finding and minimum truthful ticket

Do not add a generic “bank write followed by pop/reti is complete” exception. The smallest useful deliverable is an opt-in **ending VBlank entry contract**, printed prominently in each generated packet, plus narrowly checked interrupt terminal recognition and an exact state-19 sound-call guard. Produce COMPLETE *under that contract* packets for text $68572 and transfer $26D7; preserve the default standalone INCOMPLETE behavior. COMPLETE still means source control-flow coverage under declared preconditions, never native implementation verification.

The three actual blockers are independent:

1. Dynamic bank values are currently discarded (`ld a,[...]` makes A unknown), so $685D8 and the *initial* transfer write $26DA fail before the pops are visited.
2. An isolated entry has no VBlank register-save frame. Merely accepting the bank write exposes `pop consumes unknown caller stack` at $685E0/$2723. Existing stack tuples conflate saved registers and unknown targets; do not seed four arbitrary Nones and assume that proves interrupt semantics.
3. Generic transfer has a real conditional call at $2716 to UpdateSound $2089. Existing traversal explores both conditional-call edges and does not track CP flags. An epilogue matcher cannot eliminate this dependency. There is no native sound implementation to name as --dependency.

## Exact caller and terminal evidence

* VBlank saves registers at home.asm:99..104 / ROM $0154..$0158: `F3 F5 C5 D5 E5` = DI, PUSH AF, BC, DE, HL. Hardware interrupt return is beneath that saved frame. home.asm:127..138 / $018B..$01A4 checks active transfer ($019D JP NZ,$26D7) **before** state19 ($01A4 JP Z,$2CA3). Direct jumps preserve the saved frame.
* $2CA3..$2CAD, home.asm:5921..5925: `3E 1A EA 4E A2 EA 00 21 C3 28 45`: set both sCurROMBank ($A24E) and selected bank to $1A, jump $4528. This is stronger than an unknown caller-bank restoration assumption. $68528 checks credits phase and JP NZ,$4572; it contains no intervening write to $A24E. Its earlier HRAM DMA call is outside the requested text packet and requires its own caller seam, not invented game logic.
* $685D5..$685E4, main.asm:39961..39970: `FA 4E A2 EA 00 21 3E 01 EA 82 FF E1 D1 C1 F1 D9`. Restore selected bank from $A24E; set frame-ready $FF82=1; POP HL,DE,BC,AF; RETI. With the verified ending caller, $A24E=$1A, so the banked ROMX continuation remains bank $1A. **Without that fact, decoding suffix bytes at the old bank is unsafe**: the bank switch changes subsequent instruction fetches immediately.
* $26D7/$26DA, home.asm:5108..5109: `FA 65 A2 EA 00 21`, source data bank loaded from sVRAMTransferBank ($A265). The copy and continuation code are ROM0, so source bank changes data interpretation but not their instruction locations. This is a real data-bank effect, not a return-only suffix.
* $2712..$2727, home.asm:5150..5161: `F0 9B FE 13 C4 89 20 3E 01 E0 82 FA 4E A2 EA 00 21 E1 D1 C1 F1 D9`. State19 comparison skips CALL NZ,$2089, sets frame-ready, restores selected bank, pops saved registers, RETI. ROM0 continuation remains fetchable regardless of restored bank.
* UpdateSound $2089..$2096, home.asm:4252..4259, tests sound-disabled, may select bank4 and CALL $4030 (ROM $10030), then RET. It is neither an implemented bank-preserving native helper nor an acceptable no-op.

Source line correction: actual text entry is main.asm:39893; body is 39894..39970, not the approximate 39902 start in prior audit prose. Transfer source range 5107..5161 is exact.

## Concrete scope and contract design

Owned files for a subsequent tooling ticket: tools/asm_slice.py, tools/handler_task.py, tools/tests/test_porting_tools.py, and a small checked contract manifest only if needed. Do not touch gameplay/native modules or queue status. Generated packets can be reviewed in /tmp first.

Use a closed named contract, e.g. `--entry-contract ending-text-vblank` / `ending-transfer-vblank`, with exact supported entry addresses, exact witness byte ranges, initial memory facts, and interrupt frame description. Avoid a generic “trust arbitrary epilogue address” flag. Output the full assumptions, witnesses, excluded paths, and interrupt-exit behavior separately from implemented helper dependencies.

* Text profile: entry $68572, selected bank $1A, memory fact $A24E=$1A derived from the checked $2CA3 wrapper, four saved-register words atop an opaque interrupt-return continuation. Validate the VBlank push order and text dispatch witness. Only annotated loads of proven facts acquire constant A; the body does not overwrite $A24E. Validate exact terminal bytes and require no unbalanced local pushes/calls when entering it. Include every terminal instruction in the packet. Consume the exact typed save-frame and terminate at RETI; never follow the opaque interrupted PC. No synthetic CPU stack in native gameplay.
* Transfer profile: entry $26D7, state byte $FF9B=$13; $A24E caller bank is a profile fact (ending uses $1A). For the minimal ending requests, use a **concrete, checked source-bank fact $A265=$1A**, not general unknown-bank acceptance. Exact request producers are main.asm:39273..39288 (source $4656,dest $8000,length $0800) and 39685..39700 (source $5096,dest $9800,length $0400), each storing 26 to $A265. Expose this restriction explicitly in the packet. Existing ordinary constant bank-write semantics then suffice for $26DA/$2720.
* A third $A265 write exists in the bounded ending source at main.asm:39391..39406: bank $10, source $6000,dest $8800,length $1000. It follows an unlabelled body after an earlier RET and is not demonstrated reachable by the 20-target state table. Do not silently assert that all ending source banks are $1A or invent a substate for it. If a general transfer source packet is desired, add a separately named bank-$10 contract/packet or a finite union of explicitly witnessed alternatives, and keep reachability separate. No arbitrary unknown selected-bank execution is needed to unblock the proven ending requests.
* At $2712, validate exact `LDH A,[$9B]; CP $13; CALL NZ,$2089` bytes and unchanged state fact, then prune only the impossible taken edge under the explicit state19 contract. Either a narrowly structured guard proof or sound constant/Z tracking is sufficient. A tiny exact guard proof is the minimum ticket; a generic flags interpreter is unnecessary. The call instruction still belongs in the packet, annotated “taken edge impossible under declared entry contract.” Other states/default mode keep the sound dependency unresolved.
* For terminals, a small distinct typed interrupt-frame contract or atomic structurally validated suffix is sufficient. Never make a naked POP/RETI valid by globally relaxing stack checks. Reject mismatched pop order, additional/removed pops, RET in place of RETI, live local-call frames, skipped frame-ready write, wrong bank-source address, or changed banked continuation. If memory facts are represented generically, writes to those addresses invalidate/update them conservatively; do not treat manifest constants as immutable magic through arbitrary callees.

## Callback/interrupt-return answer

The **interrupt return** can be explicit and structurally checked: source VBlank pushes, checked direct-dispatch path, typed saved-frame contract, checked exact suffix, frame-ready write, RETI. It is a caller boundary, not a missing native helper implementation. A source packet may close its local interrupt control flow honestly without decoding the interrupted program, provided its contract is visible and enforced.

A **sound callback** can be documented as an external callback obligation with exact address, condition, side effects, and return protocol; that does not make an unimplemented native dependency implemented. There is no need for such a callback in the ending-only transfer profile because state19 proves it untaken. For a generic transfer packet covering all states, retain INCOMPLETE until sound source/dependencies are independently resolved, or define a separately labelled open source packet whose scope explicitly excludes callback internals; do not silently call the latter an ordinary closed COMPLETE slice.

## Required tests for the tooling ticket (not run in this audit)

Extend SliceTests in tools/tests/test_porting_tools.py (currently covers conditional CFG, trampoline restore, push-HL/RET, unknown computed jump/bank, budget, helper binding, source indexing).

1. Default $68572 and $26D7 remain INCOMPLETE. Unknown arbitrary bank writes and computed JP HL remain INCOMPLETE. Budget exhaustion still fails regardless of contract.
2. Positive exact fixture: checked caller witness, text entry, both phase branches, full suffix in instructions, terminal contract present; no native dependency manufactured. ROM-backed integration asserts real $68572 COMPLETE under named profile, exit0 packet generation, all terminal locations included.
3. Positive transfer fixture/profile: initial source-bank write is resolved from checked producer; both copy-loop/count branches covered; exact state19 guard records CALL instruction but omits $2089 traversal. Full suffix and terminal contract emitted. Real $26D7 ending-$1A profile COMPLETE, exit0. Optional separate bank-$10 profile succeeds without claiming state-table reachability.
4. Entry/witness tampering: wrong save order, wrong wrapper bank/store address/jump, changed request bank, changed state comparison/call target/condition, wrong terminal restore operand, missing frame-ready write, wrong POP order/count, RET instead of RETI, local PUSH imbalance or live CALL frame all reject the contract/leave INCOMPLETE (exit2).
5. State unknown/non19 explores the taken sound branch and stays INCOMPLETE; no callback/no-op invented. Exact guard fact invalidated by an intervening state store must not prune it.
6. Bank safety: text restore value different from $1A must never reuse suffix bytes from bank $1A. Missing/wrong $A24E fact rejects. Unsupported transfer source bank rejects profile, does not gain a blanket data-bank exemption. Ordinary constant cross-bank trampoline tests remain green.
7. Contract validation checks actual ROM bytes, not only source label names or presence of an existing Rust function. Every claimed witness/terminal instruction has an exact SourceIndex location. Missing symbols/source evidence or changed bytes fail closed.
8. Output tests assert contract/preconditions and scoped COMPLETE wording appear, helper list stays empty for these profiles, generic mode remains unchanged, and failed contract validation cannot produce exit0.

Acceptance: focused Python tooling tests and normal existing tool regression suite; separately regenerate/review the two scoped packets from actual ROM with no --dependency. No capture gate needed for tooling-only source extraction. Native later tickets implement text/transfer state with typed requests, progress and effects; they must not emulate bank registers, interrupt stack, or CPU registers.
