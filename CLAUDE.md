# CLAUDE.md

The rules of this repository are in `AGENTS.md`, so that they serve any
assistant alike.

@AGENTS.md

## Notes for Claude Code

- The roles in `agents/roles/` have the front matter of a subagent. To call
  one by name, link or copy its file into `.claude/agents/`.
- Review a change from a fresh context: start the reviewer with the diff and
  the pages, not with the reasoning that wrote the change.
- For a question about a dependency's API (parley, egui, wgpu, AccessKit),
  read its documentation at the version in `Cargo.lock` before answering.
