# PKU3B AI CLI Specification

## 1. Purpose

This document is the implementation and product specification for \`pku3b-ai-cli\`.

It is written for AI coding agents and human contributors who will implement the project later
without relying on prior chat context.

This version of the spec reflects the product direction that the command surface must be
comprehensive enough to be competitive, while the CLI itself must remain low-level and suitable
as a backend for AI skills.

## 2. Product Definition

### 2.1 One-sentence definition

\`pku3b-ai-cli\` is a single-binary PKU Blackboard CLI that exposes a full set of low-level
Blackboard and adjacent PKU workflow primitives for AI agents, shell scripts, and advanced users.

### 2.2 Core product promise

The project should combine three properties at the same time:

1. broad command coverage;
2. low-level, composable primitives;
3. installation-friendly local delivery.

### 2.3 What "broad but low-level" means

The product should include a wide set of resource and action commands, but each command must
remain primitive.

Examples of acceptable primitive commands:

- list assignments;
- get one announcement by ID;
- find documents by title;
- download one video's files;
- list course tree nodes;
- launch syllabus polling;
- read cache info.

Examples of non-goals that still remain outside the CLI:

- tell me what is most urgent;
- summarize this week's learning tasks;
- generate notes automatically;
- create an AI digest;
- decide what I should do next.

Those belong in skills or higher-level agents that call this CLI.

## 3. Product Goals

### 3.1 Primary goals

- ship as a single executable without requiring Python, MCP, or a long-running server;
- expose a command surface that is at least as complete as the union of useful user-facing
  \`pku3b\` features and useful structured-access \`pku3b_AI\` features;
- provide stable machine-readable Markdown output for all major read and write commands;
- support both human CLI usage and AI-skill orchestration;
- preserve low-level semantics even when command coverage is broad.

### 3.2 Secondary goals

- offer good interactive UX for direct terminal use;
- support secure local credential handling;
- support deterministic filtering and lookup patterns;
- allow future GUI/TUI/web wrappers to reuse the same core.

## 4. Competitive Positioning

This project should compete on product shape, not only on scraping access.

### 4.1 Competitive advantages to preserve

- simpler deployment than \`pku3b_AI\`;
- broader structured capability coverage than plain \`pku3b\`;
- richer command coverage than a "minimal MVP" CLI;
- deterministic AI-friendly Markdown contracts;
- local-first execution without server orchestration.

### 4.2 Competitive dimensions

The project should be stronger than its references across these dimensions:

- installation simplicity;
- command completeness;
- structured data quality;
- machine-readable output quality;
- operational safety for file actions and submissions;
- testability and automation readiness.

## 5. Non-Goals

The following still remain out of scope for the CLI itself:

- MCP server behavior;
- embedded LLM calls;
- chat UI;
- digest generation;
- prioritization;
- reminder logic;
- knowledge-base synchronization workflows;
- multi-step reasoning or planning.

The product can support those use cases indirectly by being a strong backend for AI skills.

## 6. Reference Sources

Implementation should reuse and merge ideas from these local repositories:

- \`/Users/feng/Documents/CODE/GITHUB/pku3b\`
- \`/Users/feng/Documents/CODE/GITHUB/pku3b_AI\`

### 6.1 Reuse from \`pku3b\`

Treat \`pku3b\` as the operational base. Reuse or adapt:

- login and session flow;
- OTP support;
- Blackboard-style campus-card SSO behavior should be preferred where possible: try the direct
  login path first, and only request OTP after an actual server-side OTP rejection;
- config conventions;
- cache conventions;
- assignment submission flow;
- video download flow;
- schedule support;
- syllabus support;
- Bark support;
- TTShitu support;
- thesis-lib support where retained;
- release and packaging patterns.

### 6.2 Reuse from \`pku3b_AI\`

Treat \`pku3b_AI\` as the structured-capability reference. Reuse or adapt:

- document scraping and normalization;
- announcement body and attachment normalization;
- resource-handle patterns;
- course tree construction;
- title-based find operations;
- structured resource summaries;
- cross-resource capability decomposition.

### 6.3 Do not inherit

Do not make these part of the runtime foundation:

- Python dependency;
- \`maturin\`;
- MCP server;
- Cherry Studio assumptions;
- a required frontend service.

## 7. Product Principles

### 7.1 Primitive-first

Each command performs one narrow operation even when the overall command surface is large.

### 7.2 Coverage-first

The project is allowed to expose many primitive commands if doing so preserves competitive
capability coverage.

### 7.3 Contract-first

Markdown output stability takes priority over terminal prettiness.

### 7.4 Installation-first

The deployment target is one executable plus local config and cache files managed by the app.

### 7.5 Safe-by-default

Potentially destructive, remote-mutating, or filesystem-writing behavior must be explicit,
previewable where practical, and deterministic.

## 8. Current login behavior note

- Blackboard-facing commands should behave like the teaching-site campus-card path where possible:
  try direct login first, then only request OTP after a real IAAA `E05` rejection.
- the real user flow to mirror is clicking the campus-card user entry on
  `https://course.pku.edu.cn/webapps/login/`
- for this account, normal Blackboard login is usually non-OTP; OTP should be treated as a
  conditional fallback, not as a mandatory preflight step
- This matters because the live account in this thread reports different IAAA modes for different
  app ids: `blackboard` reports no OTP, while `portalPublicQuery` reports OTP.
