<!-- Purpose: Explains how agents should retrieve and apply knowledge-base modules. -->

# 🚀 How LLMs Should Use This Knowledge Base

This document explains the intended usage pattern for AI coding agents (Claude Code, Gemini, Codex, Grok, Cursor, etc.) when working on FunnyFileManager.

## 💡 Core Principle

**All detailed project knowledge lives in `docs/knowledge-base/`.**

The agent instruction files (`AGENTS.md`, `CLAUDE.md`, `GEMINI.md`) are intentionally kept thin. They exist only to:
- Introduce the project.
- Tell you *how* to use the knowledge base.
- Add any agent-specific workflow notes.

Do **not** treat the agent .md files as the source of deep technical knowledge.

## ✅ Recommended Workflow

1. **At the beginning of any non-trivial session**, read:
   - `docs/knowledge-base/00-index.md`

2. **For any topic**, read the relevant module instead of asking the human to paste large amounts of context.

3. **Be selective** — only pull the files you actually need for the current task.

4. **When in doubt**, start with these high-value modules:
   - `03-rust-best-practices/type-system.md` (project philosophy)
   - `02-architecture/module-structure.md` (where everything lives)
   - `05-testing/testing-strategy.md`

## 🔄 Updating the Knowledge Base

If you discover missing or outdated information:
- Add or improve the content in the appropriate topic file inside `docs/knowledge-base/`.
- Do **not** duplicate the information into AGENTS.md / CLAUDE.md / GEMINI.md.
- Update the `00-index.md` if you add a new module.

This keeps the system DRY and maintainable for all future agents.

## 🏛️ Why This Structure Exists

Previous versions of the agent instructions suffered from heavy duplication of Rust best practices, project structure, and testing rules. The knowledge base approach solves this by having one canonical source that any LLM can retrieve from on demand.
