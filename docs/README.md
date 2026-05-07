# PKU Blackboard CLI Docs

This folder is the canonical planning and execution surface for the \`pku3b-ai-cli\` upgrade
inside the real \`/Users/feng/Documents/CODE/GITHUB/pku3b\` repository.

If you want the plain-language version first, read:

0. `PKU3B-AI-CLI-通俗说明.md`
   - what changed in normal language
   - what is already done
   - what the current milestone actually means

Then read the canonical planning docs in this order:

1. \`PKU3B-AI-CLI-goal.md\`
   - short \`/goal\` text
   - completion criteria
   - live-test credential handoff
2. \`PKU3B-AI-CLI-spec.md\`
   - product definition
   - target command surface
   - architecture and principles
3. \`PKU3B-AI-CLI-phase-plan.md\`
   - phased delivery logic
   - recommended milestone order
   - what to build first versus later
4. \`PKU3B-AI-CLI-reuse-audit.md\`
   - what to reuse directly from \`pku3b\`
   - what to borrow from \`pku3b_AI\`
   - what must stay out of the final runtime
5. \`PKU3B-AI-CLI-testing.md\`
   - coverage rules
   - Markdown contract expectations
   - live and mocked test guidance

## Current implementation stance

- modify the real Rust repo in \`/Users/feng/Documents/CODE/GITHUB/pku3b\`
- keep the final product single-binary and Rust-native
- do not make Python, MCP, or a long-running server part of runtime delivery
- preserve and extend the current CLI instead of rewriting the lower Blackboard stack from zero
- in short: build on the current Rust foundation and refactor upward, not a clean-slate rewrite

## Initial refactor slice already started

The first implementation slice in this thread focuses on:

- introducing shared Markdown-output helpers inside the Rust CLI layer
- adding Markdown support to selected high-value command families
- keeping the human-readable CLI behavior intact when \`--markdown\` is not requested
- allowing \`PKU_USERNAME\` and \`PKU_PASSWORD\` to bootstrap live tests without an interactive
  \`init\` step
- failing clearly in non-interactive OTP-required flows instead of attempting a hidden prompt
- allowing \`coursetable\` to fall back to Blackboard calendar data when the exact portal path is
  OTP-blocked or temporarily unavailable

Implemented so far:

- \`course --markdown\`
- \`cache --markdown\`
- \`announcement --markdown\`
- \`document --markdown\`
- \`coursetable --markdown\`
- \`assignment --markdown\`
- \`find --markdown\`
- \`search --markdown\`
- \`tree --markdown\`
- \`video --markdown\`
