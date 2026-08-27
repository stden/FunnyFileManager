<!-- Purpose: Documents the Claude command for generating and checking docs. -->

Build Rust documentation and check for issues.

Steps:

1. Run `cargo doc --no-deps 2>&1` to build documentation for this crate only.

2. If the build fails or shows warnings:
   - Show warnings about missing docs (`missing_docs`), broken intra-doc links, or malformed doc comments.
   - Fix broken `[links]` in doc comments — check that referenced items exist.
   - Fix malformed code blocks in doc comments (missing language tag, syntax errors).
   - Do NOT add boilerplate doc comments just to silence `missing_docs` — only add docs that are genuinely useful.

3. Run `cargo test --doc 2>&1` to run all documentation tests (code examples in `///` comments).

4. If any doc tests fail, fix the example code in the doc comment (not the production code).

5. Report: "Docs OK — N doc tests passed" or list issues found.

Rules:
- Doc comments should explain *why*, not just restate the function name.
- Examples in doc comments must be valid, runnable Rust code.
- Do not add `/// # Panics` or `/// # Errors` sections unless the function actually panics/errors in non-obvious ways.
