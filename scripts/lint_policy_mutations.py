"""Mutations of the compliant manifest fixtures, each with the one violation
that the policy checker must report for it (None: the mutation must pass).
"""


CLIPPY = "[lints.clippy]\n"
# (crate, text to replace, replacement, the one violation it must cause or None)
MUTATIONS = (
    (
        "tui",
        CLIPPY,
        CLIPPY + '# Too noisy.\ncast_sign_loss = "allow"\n',
        "tui: [lints.clippy] cast_sign_loss is enforced; remove it from the manifest and fix the code",
    ),
    (
        "tui",
        CLIPPY,
        CLIPPY + '# Test helpers.\nunnecessary_wraps = "allow"\n',
        "tui: [lints.clippy] unnecessary_wraps is test-scope only; allow it in test code, not here",
    ),
    (
        "mcp",
        CLIPPY,
        CLIPPY + '# Cucumber steps.\nunused_async = "allow"\n',
        "mcp: [lints.clippy] unused_async is test-scope only; allow it in test code, not here",
    ),
    (
        "mcp",
        CLIPPY,
        CLIPPY + '# Prose.\ndoc_markdown = "allow"\n',
        "mcp: [lints.clippy] doc_markdown is not part of the lint policy",
    ),
    (
        "tui",
        CLIPPY,
        CLIPPY + 'panic_in_result_fn = "deny"\n',
        "tui: [lints.clippy] panic_in_result_fn is not part of the lint policy",
    ),
    (
        "tui",
        'unwrap_used = "warn"',
        'unwrap_used = "allow"',
        'tui: [lints.clippy] unwrap_used is "allow"; the policy requires "warn"',
    ),
    (
        "mcp",
        'print_stdout = "warn"\n',
        "",
        'mcp: [lints.clippy] print_stdout is missing; the policy requires "warn"',
    ),
    (
        "colorer",
        'trivial_casts = "warn"\n',
        "",
        'colorer: [lints.rust] trivial_casts is missing; the policy requires "warn"',
    ),
    (
        "tui",
        'multiple_unsafe_ops_per_block = "warn"\n',
        "",
        'tui: [lints.clippy] multiple_unsafe_ops_per_block is missing; the policy requires "warn"',
    ),
    # A lint that judges unsafe code has nothing to judge where unsafe is forbidden.
    (
        "mcp",
        CLIPPY,
        CLIPPY + 'multiple_unsafe_ops_per_block = "warn"\n',
        "mcp: [lints.clippy] multiple_unsafe_ops_per_block is not part of the lint policy",
    ),
    (
        "tui",
        "# Every pure getter would need #[must_use]; noise without catching bugs.\n",
        "",
        'tui: [lints.clippy] must_use_candidate = "allow" needs a one-line reason comment',
    ),
    (
        "tui",
        'unsafe_code = "deny"\n',
        "",
        'tui: [lints.rust] unsafe_code is missing; the policy requires "deny"',
    ),
    (
        "tui",
        'unsafe_op_in_unsafe_fn = "deny"',
        'unsafe_op_in_unsafe_fn = "warn"',
        'tui: [lints.rust] unsafe_op_in_unsafe_fn is "warn"; the policy requires "deny"',
    ),
    (
        "tui",
        'pedantic = { level = "warn", priority = -1 }',
        'pedantic = "warn"',
        'tui: [lints.clippy] pedantic is "warn"; the policy requires { level = "warn", priority = -1 }',
    ),
    (
        "tui",
        'too_many_lines = "warn"',
        'too_many_lines = "allow"',
        'tui: [lints.clippy] too_many_lines is "allow"; the policy requires "warn"',
    ),
    (
        "mcp",
        'unsafe_code = "forbid"',
        'unsafe_code = "deny"',
        'mcp: [lints.rust] unsafe_code is "deny"; the policy requires "forbid"',
    ),
    (
        "mcp",
        'unsafe_code = "forbid"\n',
        "",
        'mcp: [lints.rust] unsafe_code is missing; the policy requires "forbid"',
    ),
    (
        "colorer",
        'undocumented_unsafe_blocks = "deny"\n',
        "",
        'colorer: [lints.clippy] undocumented_unsafe_blocks is missing; the policy requires "deny"',
    ),
    (
        "colorer",
        'missing_safety_doc = "deny"',
        'missing_safety_doc = "warn"',
        'colorer: [lints.clippy] missing_safety_doc is "warn"; the policy requires "deny"',
    ),
    (
        "colorer",
        "[lints.rust]\n",
        '[lints.rust]\nunsafe_code = "forbid"\n',
        "colorer: [lints.rust] unsafe_code is not part of the lint policy",
    ),
    # Dropping an allowance only makes a crate stricter.
    (
        "tui",
        '# Nursery style lint that fights `match`/`if let` readability.\noption_if_let_else = "allow"\n',
        "",
        None,
    ),
)
