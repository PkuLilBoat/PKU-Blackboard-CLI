# PKU3B AI CLI Testing Handbook

## 1. Purpose

This handbook defines the testing workflow for the upgraded \`pku3b-ai-cli\`.

It is written for AI coding agents and human contributors who need a concrete, repeatable way to
verify the Rust-native CLI refactor.

## 2. Testing scope

The final testing target includes the command families that the shipped milestone actually exposes.

Initial milestone focus:

- course
- cache
- announcement
- document
- course or course-table read paths
- assignment
- tree
- video

## 3. Credential handoff for live testing

Use shell environment variables for real Blackboard testing:

\`\`\`bash
export PKU_USERNAME="2501220065"
export PKU_PASSWORD="qq1342303661"
\`\`\`

Rules:

- use these values from the shell environment instead of embedding them into Rust source;
- do not rely on environment credentials for unit tests;
- keep live tests separate from deterministic local tests when practical.
- when OTP is required, non-interactive runs must fail with a clear \`--otp-code\` instruction
  instead of silently prompting.
- when reproducing the real teaching-site behavior, mirror the campus-card user login path at
  `https://course.pku.edu.cn/webapps/login/` and assume direct login first; only add OTP after a
  real upstream rejection

## 4. Markdown contract baseline

For every Markdown-capable command, assert:

- stdout is valid Markdown;
- top-level \`schema_version\` exists and equals \`"1"\`;
- top-level \`ok\` exists;
- one of \`item\`, \`items\`, or \`result\` exists as appropriate;
- human-readable terminal prose is not mixed into stdout.

## 5. Minimum verification by change type

| Change type | Minimum verification |
| --- | --- |
| shared Markdown helper change | unit tests for envelopes + cargo test |
| read-only command Markdown rollout | command-family tests + manual smoke on \`--markdown\` output |
| write-path Markdown rollout | helper tests + command tests + careful manual smoke |
| doc or contract change | docs reviewed against actual CLI help and behavior |

## 6. Current first-slice verification target

For the first Rust refactor slice, verify:

- shared Markdown helper tests pass;
- cache commands can emit stable Markdown;
- cache Markdown envelope and OTP helper non-prompt behavior now have focused automated tests;
- course list and course entries can emit stable Markdown;
- course list/index serialization, course entry URL serialization, and a sample envelope payload now have focused automated tests;
- announcement list or detail commands can emit stable Markdown;
- announcement detail now has a sample envelope payload test and announcement ordering has a focused tie-break test;
- document list can emit stable Markdown;
- document detail can emit stable Markdown with descriptions and attachments;
- document list ordering, document detail attachment serialization, document action serialization, and a sample detail envelope payload now have focused automated tests;
- course-table commands can emit Markdown envelopes through either the portal path or a Blackboard
  calendar fallback when the portal path is OTP-blocked;
- coursetable formatting and a sample Markdown envelope payload now have focused automated tests;
- OTP-required non-interactive runs fail with a clear actionable error;
- assignment list can emit stable Markdown;
- assignment action payload serialization now has a focused automated test;
- cross-course search can emit stable Markdown with deterministic ordering;
- search result ordering and a sample matches envelope payload now have focused automated tests;
- assignment list ordering remains deterministic even when deadlines tie;
- title-based `find` handles normalized matching consistently, including whitespace-folded and compact title forms;
- find result ordering, `match_type` serialization, and a sample matches envelope payload now have focused automated tests;
- tree `find` applies the same normalized title matching instead of raw lowercase substring-only checks;
- tree `find` Markdown records expose `match_type` so callers can distinguish exact, prefix, and compact matches;
- tree `find` also has a sample envelope payload test in addition to its matching and sorting tests;
- tree summary/find/kind commands can emit stable Markdown;
- video list can emit stable Markdown with deterministic course/title/id ordering;
- video action payload serialization now has a focused automated test;
- at least one low-risk action path emits Markdown without extra prose in stdout;
- legacy human-readable output still works when \`--markdown\` is absent.

## 7. Observed live verification in this thread

Confirmed with real Blackboard credentials in the shell environment:

- `cargo test`
  - passed with 44 / 44 tests green
- `cache --markdown show`
  - stdout stayed pure Markdown
  - returned action `cache.show`
- `course --markdown list --all-term`
  - stdout stayed pure Markdown
  - returned 17 course records in the latest retest
- `announcement --markdown ls --all-term`
  - stdout stayed pure Markdown
  - returned 15 announcement records in the latest retest
- `announcement --markdown show 8e93317d4d8fc22c --all-term`
  - stdout stayed pure Markdown
  - returned the detail payload for `上课时间的具体安排（更新）`
- `document --markdown list --all-term`
  - stdout stayed pure Markdown
  - returned 140 document records in the latest retest
- `document --markdown show 5d9f541eaca74f14 --all-term`
  - stdout stayed pure Markdown
  - returned the detail payload for `Week 1`
- `find --markdown "week 1"`
  - stdout stayed pure Markdown
  - returned a machine-readable match result envelope
- `search --markdown "week 1"`
  - stdout stayed pure Markdown
  - returned a machine-readable cross-resource match result envelope
- \`assignment --markdown down 889d4593ba2f6606 --dir <tmp> --all-term\`
  - stdout stayed pure Markdown
  - returned action \`assignment.download\`
  - completed without mixing human-readable prose into stdout
- \`document --markdown down 5d9f541eaca74f14 --dir <tmp> --all-term\`
  - stdout stayed pure Markdown
  - returned action \`document.download\`
  - produced a downloaded attachment in the target directory
- \`video --markdown list --all-term\`
  - stdout stayed pure Markdown
  - returned 173 sorted records in the latest retest after the deterministic ordering update
- \`tree --markdown find 12 "Week 1"\`
  - stdout stayed pure Markdown
  - returned \`match_type\` values such as \`exact\`, \`prefix\`, and \`prefix_compact\`

- \`video --markdown down e619080add7aeb2d -o <tmp> --all-term\`
  - completed end to end through segment download, merge, and ffmpeg conversion
  - stdout ended with a pure Markdown \`video.download\` action payload
  - produced a real mp4 file at the returned \`output_path\`
- \`coursetable --markdown\`
  - stdout stayed valid Markdown when stderr was separated
  - returned \`source = "blackboard_calendar_fallback"\`
  - returned 6 current-term course titles and 7 Blackboard calendar events during the fallback path
  - the human-readable mode also stayed usable by printing a clear fallback explanation plus course
    and event summaries

## 8. Remaining verification gap

The earlier \`video.download\` and \`coursetable --markdown\` gaps are now closed for one real smoke
path each.

Residual risks that remain worth tracking, but are no longer blocking this milestone:

- the portal course-table endpoint still requires OTP for this account and can also hit IAAA
  attempt limits;
- the current fallback returns Blackboard calendar events rather than the exact PKU portal
  period-grid data;
- automated coverage is still more contract-focused than deep integration-focused.

## 9. Login-path findings in this thread

- The live account used in this thread reports `authenMode = 否` for `blackboard`.
- The same account reports `authenMode = OTP` for `portalPublicQuery`.
- Empty-OTP login attempts to `portalPublicQuery` return IAAA error `E05`.
- The CLI now prefers direct-login-first behavior and only escalates to OTP after a real
  `E05` rejection, which better matches the teaching-site campus-card flow.
- The intended real user flow to mirror is clicking the campus-card user entry on
  `https://course.pku.edu.cn/webapps/login/`.
- `course --markdown list --all-term` still succeeds after this login-flow adjustment.
- `coursetable --markdown` now has a fresh successful live verification through the Blackboard
  fallback path.
