## c2s-len10-disambig — Disambiguate the C2S len=10 / 3607-frame bucket among 3 candidates

## Role & workdir
Pure-Python protocol-analysis worker. Workdir: `/home/sdancer/dark-december-c2s-len10-disambig` (worktree of `/home/sdancer/darkdecember/`, branch `c2s-len10-disambig`).

## Current goal / sub-goal
- **goal_key**: `dark_december_c2s_len10_binding` (new)
- **sub_goal_key**: `pick-correct-rq-among-3-candidates-for-c2s-len10`

## Why this turn exists
The C2S len=10 bucket holds **3607 frames** (28.3% of all C2S frames). Cycle-412's rz-typename-binding marked it "low confidence" with 3 candidates whose 2-byte Serialize bodies all match: `FRzCharacterLookVisibleChangeRq`, `FRzCoContentGiveUpAcceptRq`, `FRzApplyProjectileRq(count=0)`. The bucket has **93.9% dominant decoded prefix `12030000`**, suggesting a single typename owns it. Disambiguating closes the second-largest unbound C2S class.

## Hypothesis
The 3607 frames are dominated by ONE specific typename (not split among the 3 candidates). Two orthogonal disambiguation strategies:

**Strategy A — Sibling Rp check**: each candidate Rq has a paired Rp class (`FRzCharacterLookVisibleChangeRp`, `FRzCoContentGiveUpAcceptRp`, `FRzApplyProjectileRp`). For each candidate, check whether its Rp exists in the GS_CL catalog AND appears in the S2C stream at an expected length AND fires within ±20 ordinals of the C2S len=10 frame. The candidate whose Rp shows the strongest temporal lift is the winner.

**Strategy B — Packet ID resolution**: each candidate has a unique 2-byte `msg_type` packet ID retrievable from its `GetType` method in the dump. Compare those IDs to the 2-byte decoded prefix of the len=10 frames (the 4-byte prefix `12030000` corresponds to 2-byte packet_id `0x0312` after the +8 framing model). Best match wins.

## Falsification (3 outcomes)
- (a) **One candidate produces a lift ≥ 5 against its sibling Rp AND its packet_id matches the dominant prefix** → SUCCESS. Fact: `dark_december_c2s_len10_bound_<typename>`.
- (b) **No candidate's packet_id matches AND no Rp shows lift ≥ 5** → all 3 candidates are wrong; len=10 belongs to a 4th typename we haven't enumerated. Fact: `dark_december_c2s_len10_unknown_typename`.
- (c) **Partial** — packet_id matches one candidate but Rp doesn't pair, OR vice versa → name the most likely candidate with caveats. Fact: `dark_december_c2s_len10_partial_<typename>`.

## Success criteria — SINGLE TURN, do all before stopping

**Primary**: write `/home/sdancer/dark-december-c2s-len10-disambig/analysis/c2s_len10_disambig_2026-05-15.md` with:

1. **Resolve packet IDs**: search the dynsym shard for `GetType` returns of the 3 candidates. The cycle-415 example shows `FRzMoveRq::GetType returns 0x0386`. Find:
   - `CL_GS::FRzCharacterLookVisibleChangeRq::GetType`
   - `CL_GS::FRzCoContentGiveUpAcceptRq::GetType`
   - `CL_GS::FRzApplyProjectileRq::GetType`
   - Disassemble each (short, ≤10 insns: ret with `mov w0, #imm`).

2. **Compute observed packet_id from wire prefix**: the cycle-408 s2c-inventory documented C2S len=10 dominant prefix `12030000` (decoded). Per cycle-412 +8 framing, the 2-byte packet_id is at raw offset 6..7 (before adjacent-XOR). Compute the raw packet_id for each of the 3607 len=10 frames; report the distribution.

3. **Cross-reference packet_id vs candidate GetType values**. Identify the single candidate whose `GetType` returns the observed packet_id.

4. **Strategy A sibling Rp check**: for each candidate, look up the Rp at `GS_CL::FRz<stem>Rp::GetType`, compute its predicted wire length, identify the S2C length-class bucket it would land in, then compute temporal lift between C2S-len=10 frames and that S2C class within ±20 ordinals on the byte-fraction timeline (use cycle-412's `c2s_s2c_pairs.csv`).

5. **Disasm sanity check**: pull the 2-byte Serialize body field from each candidate's `Serialize` disasm. If only one writes a `u16` from a specific struct offset, that field's interpretation hints at the field semantics (e.g., `bool visible` for `FRzCharacterLookVisibleChangeRq`).

6. **Verdict** (a)/(b)/(c) + closing fact via `harness fact-set`. Print `C2S_LEN10_DISAMBIG_DONE` on the final line.

## Execution flow

**Step 1** — Find + disasm the 3 GetType methods (each is tiny). Use prior worker artifacts at `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_build.py` for the symbol extraction helper.

**Step 2** — Extract observed packet_id from len=10 frames using the +8 wire model.

**Step 3** — Match packet_id to candidate GetType. Report the single winner (if exact match).

**Step 4** — Run Strategy A as cross-check.

**Step 5** — Write artifact + fact-set + DONE.

## Constraints & gotchas
- **HARD memory budget: 250 MB.** All inputs are small.
- **NO full-shard capstone disasm.** Each GetType method is ≤10 insns; bound the disasm to ≤256 bytes per method, NOT the whole .text shard. (Lesson from cycle 415's 22 GB OOM.)
- **NO new memdump. NO Frida. NO live device.**
- **HARD output cap**: artifact ≤300 lines.
- **ONE Codex turn budget: ≤20 min wall time. SINGLE-TURN COMPLETION REQUIRED**.
- Memdump dynsym shard: `/home/sdancer/dark-december-libue4-memdump/memdump/6cdd243000.bin` (image-relative VAs)
- Memdump .text shard: `/home/sdancer/dark-december-libue4-memdump/memdump/6ce4bd4000.bin` (live VA base `0x6ce4bd4000`)
- Cross-pollination memory: [[project_dark_december_wire_framing_plus8]] documents the +8 outer frame model. [[feedback_check_prior_worktrees]] documents the prior-art sweep step.

## Relevant files / references
- Worktree: `/home/sdancer/dark-december-c2s-len10-disambig/` (branch `c2s-len10-disambig`).
- Streams: `/home/sdancer/orchestrator/streams/first_quest/first_quest_c2s.tcpstream.bin`
- Memdump: `/home/sdancer/dark-december-libue4-memdump/memdump/{6cdd243000.bin,6ce4bd4000.bin}`
- RZ binding (cycle 412): `/home/sdancer/dark-december-rz-typename-binding/analysis/rz_binding_2026-05-15.md` + `rz_binding_build.py`
- C2S decoder + minor classes (cycle 397): `/home/sdancer/dark-december-c2s-minor-classes/analysis/c2s_minor_classes_2026-05-15.md`
- C2S↔S2C join (cycle 412): `/home/sdancer/dark-december-c2s-s2c-join/analysis/c2s_s2c_pairs.csv`
- 822-typename catalog: `/home/sdancer/orchestrator/analysis/dd_packet_typenames_2026-05-15.txt`
- Decoder: `/home/sdancer/orchestrator/darkdec_decoder.py`
- success-fact key: `dark_december_c2s_len10_bound_<typename>` (a)
- block-fact keys: `dark_december_c2s_len10_unknown_typename` (b), `dark_december_c2s_len10_partial_<typename>` (c)
