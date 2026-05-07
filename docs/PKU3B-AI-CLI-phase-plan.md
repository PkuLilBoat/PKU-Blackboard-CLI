# PKU3B AI CLI Phased Plan

This document turns the product spec into a phase-by-phase delivery plan.

## 1. Recommended build strategy

Create a new Rust CLI product layer inside the existing repository instead of continuing to grow
the Python + MCP orchestration path.

Recommended shape:

- keep the current Rust runtime in \`pku3b/\` as the lower operational base;
- keep \`pku3b_AI\` as a reference for structured capability and JSON contract design;
- extend the real Rust CLI progressively instead of routing through Python or MCP.

## 2. Phase map

### Phase 0: lock the product boundary

Goal:

- freeze the product promise before code spreads into the wrong layer.

Outputs:

- canonical docs in this \`docs/\` folder;
- milestone definition for the first implementation slice;
- shared JSON envelope rules;
- exit-code expectations;
- credential and config policy.

Definition of done:

- no new final-product logic is added to Python/MCP as a dependency of the target CLI;
- the first Rust implementation slice is chosen and documented.

### Phase 1: establish shared contract primitives

Goal:

- create the Rust-side helpers that make broad JSON rollout consistent.

Outputs:

- shared JSON envelope helpers;
- one or more command families upgraded to structured JSON;
- tests for the shared JSON shape.

Definition of done:

- at least one command family can emit both human-readable output and stable JSON.

### Phase 2: migrate high-value structured content

Goal:

- bring the strongest structured capabilities into the Rust runtime.

Priority families:

- announcement
- document
- tree
- richer course summary and lookup

Definition of done:

- the new Rust path expresses core structured reads that previously only existed in \`pku3b_AI\`.

Execution notes:

- prefer reusing existing Blackboard fetch logic from \`pku3b\` and changing the shaping layer
  instead of replacing the lower runtime;
- normalize document, announcement, and tree payloads into stable JSON rather than adding new ad
  hoc prose output;
- keep course-scoped IDs and handles explicit so later write-path commands can reuse them.

### Phase 3: unify deterministic query behavior

Goal:

- make \`get\`, \`find\`, and \`search\` stable enough for AI agents and shell scripts.

Outputs:

- deterministic match ordering;
- explicit not-found and ambiguous-match behavior;
- consistent JSON envelopes across query families.

Execution notes:

- harden title matching with normalization rules such as whitespace folding and case folding where
  safe;
- keep query behavior stable enough for shell scripts and higher-level AI agents;
- document match semantics in help text and JSON payload fields when ambiguity matters.

### Phase 4: extend write-path contracts

Goal:

- preserve existing download/submit strengths while exposing machine-readable action results.

Outputs:

- JSON action envelopes for downloads and submissions;
- explicit target paths and side-effect summaries;
- safety flags or preview paths where appropriate.

### Phase 5: harden delivery and release

Goal:

- make the refactor actually shippable as a local-first single binary.

Outputs:

- synchronized README/docs;
- integration tests and smoke commands;
- updated release guidance.

## 3. Recommended first milestone

Start with this order:

1. shared JSON helpers
2. cache JSON
3. announcement JSON
4. course/coursetable JSON
5. assignment and video JSON

Why:

- it adds the contract layer first;
- it exercises both read-only metadata and richer content detail;
- it avoids a giant all-at-once rewrite.

Current status in this thread:

- step 1 is done
- step 2 is done
- step 3 is done
- step 4 is done for \`coursetable\`
- step 5 is done for list surfaces and one low-risk assignment download action surface
- document list support is now done in Rust
- document detail support is now done in Rust
- tree summary/find/kind support is now done in Rust
- course list and entry-link support is now done in Rust
- deterministic cross-resource \`search\` is now done in Rust
- normalized deterministic \`find\` matching is now done in Rust, including whitespace-folded and compact title matching
- tree title lookup now reuses the same normalized query behavior for AI-facing JSON reads and emits `match_type`
- assignment and video JSON list surfaces now use explicit deterministic tie-break ordering instead of fetch-order-only output
- real JSON smoke evidence now exists for assignment.download, document.download, and video.download
- next recommended slice is broader automated tests and richer write-path verification

## 4. Live test handoff

For local live verification:

\`\`\`bash
export PKU_USERNAME="2501220065"
export PKU_PASSWORD="qq1342303661"
\`\`\`

Use shell environment variables rather than hard-coding secrets into Rust source.
