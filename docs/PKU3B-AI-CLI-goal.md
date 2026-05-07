# PKU3B AI CLI Goal Prompt

## 1. Short \`/goal\` description

Use this as the thread goal:

\`\`\`
Build pku3b-ai-cli into a single-binary, low-level PKU Blackboard CLI that merges the useful command coverage of pku3b and the structured resource capabilities of pku3b_AI, emits stable JSON for major read and write commands, and does not depend on Python, MCP, or a long-running server.
\`\`\`

This goal is intentionally short. The execution details should stay in repo docs so the agent
does not overload the goal field with step-by-step instructions.

## 2. Execution brief

When a new agent starts work, apply these constraints:

1. treat \`pku3b/\` as the operational Rust base and main code owner;
2. treat \`pku3b_AI\` as the structured-capability reference, not the runtime base;
3. do not make Python, \`pku3b_py\`, MCP, Cherry Studio, or a long-running service part of the
   final CLI runtime;
4. keep the surface broad but primitive:
   - one command = one narrow operation
   - deterministic matching and filtering
   - explicit file-writing and remote-mutating actions
   - stable machine-readable JSON output
5. preserve direct human CLI usability:
   - good help text
   - clear exit behavior
   - readable default terminal output when \`--json\` is not requested

Implementation direction:

- modify and extend the existing Rust CLI foundation;
- do not restart the Blackboard runtime from zero;
- keep correct existing behavior where possible and add stable JSON contracts around it.

## 3. Concrete completion criteria

The objective is only complete when all of the following are true:

- a single Rust binary exists for the upgraded CLI;
- the binary runs without Python, MCP, or a sidecar server;
- the implemented milestone command families are documented explicitly;
- major shipped read and write commands expose stable JSON contracts;
- tests cover the shipped command surface at the level promised by
  \`docs/PKU3B-AI-CLI-testing.md\`;
- the docs in this folder match the actual implementation.

## 4. Live test credential handoff

Use environment variables for real Blackboard testing:

\`\`\`bash
export PKU_USERNAME="2501220065"
export PKU_PASSWORD="qq1342303661"
\`\`\`

If a future agent needs to run one-shot live commands, it should source credentials from the
shell environment rather than hard-coding them into Rust source files.

Live-login note for this account:

- the real PKU teaching-site login entry is `https://course.pku.edu.cn/webapps/login/`
- prefer the campus-card user path first
- in normal Blackboard flows for this account, OTP is usually not required unless the upstream
  app id explicitly rejects the login with an OTP-only response
