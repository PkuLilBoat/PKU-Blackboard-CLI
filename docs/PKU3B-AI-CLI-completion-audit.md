# PKU3B AI CLI Completion Audit

This file is the current completion audit for the `pku3b-ai-cli` refactor.

It is not a release note. It is a requirement-to-evidence checklist that answers one question:
can the current thread already claim the upgrade is complete?

Current answer: complete for the documented Rust milestone.

## 1. Objective restated as concrete deliverables

The thread objective is only complete when all of the following are true:

1. the planning and execution docs are canonical, structured, and live in `pku3b/docs/`;
2. the real implementation base is the Rust repo in `/Users/feng/Documents/CODE/GITHUB/pku3b`;
3. the target product remains a single-binary, low-level PKU Blackboard CLI;
4. the upgraded CLI merges useful `pku3b` coverage with the structured resource behaviors borrowed from `pku3b_AI`;
5. major shipped read and write commands expose stable Markdown contracts;
6. the runtime does not depend on Python, MCP, or a long-running server;
7. the shipped surface is verified strongly enough that the docs and tests support the completion claim.

## 2. Prompt-to-artifact checklist

| Requirement | Evidence | Status |
| --- | --- | --- |
| docs were reworked into a guided, structured set | `docs/README.md`, `docs/PKU3B-AI-CLI-goal.md`, `docs/PKU3B-AI-CLI-spec.md`, `docs/PKU3B-AI-CLI-phase-plan.md`, `docs/PKU3B-AI-CLI-reuse-audit.md`, `docs/PKU3B-AI-CLI-testing.md` | done |
| docs were moved into the real repo | docs now live under `pku3b/docs/` and are referenced as canonical in `docs/README.md` | done |
| implementation is based on the Rust repo, not the Python stack | `docs/PKU3B-AI-CLI-goal.md`, `docs/PKU3B-AI-CLI-reuse-audit.md`, `src/cli/mod.rs`, new Rust command files under `src/cli/` | done |
| single-binary, no Python/MCP runtime dependency | current implementation changes are inside the Rust crate; no runtime Python bridge was added | done for current slice |
| major read commands expose stable Markdown | `course`, `cache`, `announcement`, `document`, `coursetable`, `find`, `search`, `tree`, `video`, `assignment` all have `--markdown` paths in Rust | done for current milestone |
| major write commands expose stable Markdown | verified action payloads exist for `assignment.download`, `document.download`, `video.download`, and each has a real smoke path recorded in `docs/PKU3B-AI-CLI-testing.md` | done for current milestone |
| tests and verification support the claim | focused unit tests now cover announcement/course/document/find/search/tree/assignment/video/coursetable contracts, selective cargo tests pass, and multiple live smokes are documented | done for current milestone |

## 3. Concrete evidence by area

### 3.1 Docs and source-of-truth routing

- canonical docs index: `docs/README.md`
- short goal prompt and completion criteria: `docs/PKU3B-AI-CLI-goal.md`
- product and architecture spec: `docs/PKU3B-AI-CLI-spec.md`
- phased execution map: `docs/PKU3B-AI-CLI-phase-plan.md`
- runtime reuse boundary: `docs/PKU3B-AI-CLI-reuse-audit.md`
- testing contract and live evidence: `docs/PKU3B-AI-CLI-testing.md`

### 3.2 Shared Markdown contract layer

- shared helper: `src/cli/markdown_output.rs`
- envelope contract: top-level `schema_version = "1"`, `ok`, and `item` or `items`
- cache envelope and OTP helper checks live in `src/cli/mod.rs`
- focused unit tests recorded in this thread:
  - `cargo test markdown_output -- --nocapture`
  - `cargo test cli::tests -- --nocapture`
  - current observed results: all passing

### 3.3 Query normalization and deterministic lookup

- shared normalization helper: `src/cli/query_match.rs`
- `find` uses normalized match ranking and returns `match_type`: `src/cli/cmd_find.rs`
- `search` uses normalized title and description matching: `src/cli/cmd_search.rs`
- `tree find` now reuses normalized matching and returns `match_type`: `src/cli/cmd_tree.rs`
- focused unit tests recorded in this thread:
  - `cargo test query_match -- --nocapture`
  - `cargo test cmd_find -- --nocapture`
  - `cargo test cmd_tree -- --nocapture`
  - `cargo test cmd_search -- --nocapture`
  - current observed results: all passing

### 3.4 Deterministic list ordering updates

- course entry sorting plus course record serialization tests: `src/cli/cmd_course.rs`
- document Markdown list ordering, detail attachment serialization, and action payload serialization tests: `src/cli/cmd_document.rs`
- assignment Markdown tie-break ordering and action payload serialization tests: `src/cli/cmd_assignment.rs`
- video Markdown tie-break ordering and action payload serialization tests: `src/cli/cmd_video.rs`
- focused unit tests recorded in this thread:
  - `cargo test cmd_course -- --nocapture`
  - `cargo test cmd_assignment -- --nocapture`
  - `cargo test cmd_document -- --nocapture`
  - `cargo test cmd_video -- --nocapture`
  - current observed results: all passing

### 3.5 Live Markdown smoke evidence

Documented in `docs/PKU3B-AI-CLI-testing.md`:

- `cache --markdown show`
- `course --markdown list --all-term`
- `announcement --markdown ls --all-term`
- `course list --all-term`
- `assignment --markdown down 889d4593ba2f6606 --dir <tmp> --all-term`
- `document --markdown down 5d9f541eaca74f14 --dir <tmp> --all-term`
- `video --markdown list --all-term`
- `tree --markdown find 12 "Week 1"`
- `video --markdown down e619080add7aeb2d -o <tmp> --all-term`
- `coursetable --markdown` with the Blackboard calendar fallback path

Observed outcomes in this thread:

- `cargo test` passed with 44 / 44 tests green in the latest retest;
- `cache --markdown show`, `course --markdown list --all-term`, `announcement --markdown ls --all-term`,
  `announcement --markdown show 8e93317d4d8fc22c --all-term`, `document --markdown list --all-term`,
  `document --markdown show 5d9f541eaca74f14 --all-term`, `find --markdown "week 1"`,
  `search --markdown "week 1"`, and `tree --markdown find 12 "Week 1"` all returned valid Markdown in the
  latest retest;
- action commands returned pure Markdown on stdout;
- `document.download` produced a real downloaded attachment;
- `video.download` completed through segment download, merge, ffmpeg conversion, and produced a real mp4 file;
- `coursetable --markdown` now succeeds without Python, MCP, or a sidecar service even when portal
  OTP blocks the exact portal grid path, because the Rust CLI falls back to Blackboard calendar
  data with a stable Markdown envelope.

## 4. Residual risks after milestone completion

The documented milestone is complete, but these residual risks still remain:

1. automated coverage is materially stronger than earlier in the thread, but it is still uneven across the expanded command surface;
2. current tests remain more contract-focused than deep integration-focused;
3. the portal endpoint itself is still OTP-gated for this account, so the fallback path may continue to be the practical runtime path unless a trusted portal session or valid OTP is available.

## 5. Why completion is now acceptable

The completion claim is now supportable because:

1. the canonical docs live in the real Rust repo and match the implemented command surface;
2. major shipped read and write command families expose stable Markdown through Rust-only runtime paths;
3. `coursetable --markdown` now has both focused tests and a fresh live smoke path, even when the
   exact portal endpoint is blocked, via the documented Blackboard fallback;
4. the CLI remains a single binary and does not depend on Python, MCP, or a long-running server.

This does not mean every future enhancement is done; it means the milestone defined by the docs in
this folder is now achieved.

## 6. Login-path delta from the latest verification

- `src/api/low_level/iaaa.rs` now preserves typed IAAA login errors such as `E05` instead of
  flattening them into a generic failure string too early.
- `src/cli/mod.rs` and `src/cli/cmd_course_table.rs` now use direct-login-first plus
  OTP-on-`E05` retry instead of static OTP preflight prompting.
- The intended real user flow is the teaching-site campus-card user login path at
  `https://course.pku.edu.cn/webapps/login/`, and current live behavior still aligns with that
  direct-login-first expectation for Blackboard-facing commands.
- Blackboard smoke remained healthy after the change: `course --markdown list --all-term` returned
  valid Markdown with `schema_version = "1"`, `ok = true`, and 17 items.
- The completion state is now acceptable for the milestone because `coursetable --markdown` has a
  fresh successful live verification through the Blackboard fallback path.
