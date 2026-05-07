# PKU3B AI CLI Reuse Audit

This document defines what should be reused directly from the current implementation and what
should only be borrowed as design input from \`pku3b_AI\`.

## 1. Bottom-line recommendation

The final \`pku3b-ai-cli\` should be built on top of the Rust \`pku3b\` layer, not on top of the
Python + MCP orchestration path.

Use this rule:

- reuse operational logic from \`pku3b\`;
- borrow structured models and retrieval patterns from \`pku3b_AI\`;
- do not make \`pku3b_py\` or \`pku3b_ai\` runtime dependencies of the final CLI binary.

## 2. Directly reusable from \`pku3b\`

Already strong foundations:

- login and session boot flow;
- config read/write conventions;
- cache size and cleanup flow;
- assignment list/download/submit flow;
- video list/download flow;
- course table access;
- syllabus, Bark, TTShitu, and thesis-lib integrations.

Implication:

- \`pku3b\` should remain the operational source of truth for real runtime behavior.

## 3. Borrow from \`pku3b_AI\`, do not inherit as runtime

Borrow these ideas from the AI-oriented implementation and docs:

- document content scraping and normalization;
- announcement body and attachment normalization;
- course tree summary and traversal patterns;
- title-based find flows for content families;
- structured course summary and entry shaping;
- stable Markdown envelopes for detail reads and action results;
- consistent not-found payload style.

Implication:

- the Rust runtime should learn these shapes, but should not call through Python or MCP to get
  them.

## 4. Keep out of the final runtime

Do not make these part of the final CLI runtime:

- \`pku3b_py\`;
- \`pku3b_ai/mcp_pku3b_server.py\`;
- MCP transport assumptions;
- Cherry Studio or other frontend-service assumptions.

Reason:

- the product target is a single executable without Python, MCP, or a long-running server.

## 5. First implementation consequence

The first implementation slice should:

1. add shared Markdown helpers inside the Rust CLI layer;
2. apply them to a small number of high-value command families;
3. preserve current human-readable output for terminal users;
4. avoid any runtime dependency on the Python stack.
