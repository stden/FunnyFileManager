"""The lint policy that scripts/test_lint_policy.py holds each Cargo manifest to.

COMMON is what every crate's [lints] tables must declare, UNSAFE adds each
crate's levels for unsafe code, and ALLOWANCES, ENFORCED and TEST_SCOPE
decide what else a manifest may hold.
"""


# Reliability lint families, each lint at "warn" (the gate's -D warnings makes
# it an error). Rust guards nothing trips today:
RUST_GUARDS = (
    "unused_lifetimes",
    "single_use_lifetimes",
    "redundant_lifetimes",
    "explicit_outlives_requirements",
    "unused_macro_rules",
    "meta_variable_misuse",
    "unused_import_braces",
    "unused_extern_crates",
    "macro_use_extern_crate",
    "trivial_numeric_casts",
    "unit_bindings",
    "keyword_idents_2024",
    "non_ascii_idents",
)
# Rust lints for code that hides what it does:
RUST_IDIOMS = (
    "let_underscore_drop",
    "trivial_casts",
    "unnameable_types",
    "unused_qualifications",
    "redundant_imports",
    "elided_lifetimes_in_paths",
)
# Clippy restriction lints. clippy.toml exempts tests from the panic family.
PANICS = (
    "unwrap_used",
    "expect_used",
    "panic",
    "todo",
    "unimplemented",
    "unreachable",
    "exit",
    "indexing_slicing",
    "string_slice",
    "as_conversions",
)
LOST_ERRORS = ("map_err_ignore", "let_underscore_must_use", "unused_result_ok")
TERMINAL = (
    "print_stdout",
    "print_stderr",
    "dbg_macro",
    "use_debug",
    "iter_over_hash_type",
    "filetype_is_file",
)
SAFETY_NOTES = (
    "unnecessary_safety_comment",
    "unnecessary_safety_doc",
    "allow_attributes_without_reason",
)
GUARDS = (
    "mem_forget",
    "rc_buffer",
    "rc_mutex",
    "clone_on_ref_ptr",
    "mutex_atomic",
    "mutex_integer",
    "lossy_float_literal",
    "float_cmp_const",
    "empty_drop",
    "verbose_file_reads",
    "str_to_string",
    "string_add",
    "infinite_loop",
    "large_include_file",
    "get_unwrap",
    "try_err",
    "rest_pat_in_fully_bound_structs",
    "mod_module_files",
    "needless_raw_strings",
    "unnecessary_self_imports",
    "redundant_type_annotations",
    "deref_by_slicing",
)
CARGO = ("wildcard_dependencies", "negative_feature_names", "redundant_feature_names")
# Judge unsafe blocks and attributes, so only where unsafe code can exist:
# mcp/ forbids unsafe_code, which already rejects both.
UNSAFE_OPS = {
    ("rust", "unsafe_attr_outside_unsafe"): "warn",
    ("clippy", "multiple_unsafe_ops_per_block"): "warn",
}

# Every crate: the three groups, the readability lints, honest `#[expect]`s,
# and the reliability families.
GROUP = {"level": "warn", "priority": -1}
READABILITY = ("cognitive_complexity", "excessive_nesting", "too_many_lines")
CLIPPY_FAMILIES = PANICS + LOST_ERRORS + TERMINAL + SAFETY_NOTES + GUARDS + CARGO
COMMON = {
    **{("clippy", group): GROUP for group in ("all", "pedantic", "nursery")},
    **{("clippy", lint): "warn" for lint in READABILITY + CLIPPY_FAMILIES},
    ("rust", "unfulfilled_lint_expectations"): "deny",
    **{("rust", lint): "warn" for lint in RUST_GUARDS + RUST_IDIOMS},
}
UNSAFE = {
    "tui": {
        ("rust", "unsafe_code"): "deny",
        ("rust", "unsafe_op_in_unsafe_fn"): "deny",
        ("clippy", "undocumented_unsafe_blocks"): "deny",
        **UNSAFE_OPS,
    },
    "mcp": {("rust", "unsafe_code"): "forbid"},
    "colorer": {
        ("rust", "unsafe_op_in_unsafe_fn"): "deny",
        ("clippy", "undocumented_unsafe_blocks"): "deny",
        ("clippy", "missing_safety_doc"): "deny",
        **UNSAFE_OPS,
    },
}
# The only crate-wide allowances; each needs a one-line reason comment.
ALLOWANCES = {
    "module_name_repetitions",
    "must_use_candidate",
    "missing_errors_doc",
    "missing_const_for_fn",
    "option_if_let_else",
}
ENFORCED = {
    "cast_possible_truncation",
    "cast_sign_loss",
    "cast_precision_loss",
    "struct_excessive_bools",
    "wildcard_imports",
    "similar_names",
    "missing_panics_doc",
    "items_after_test_module",
}
TEST_SCOPE = (
    "unnecessary_wraps",
    "needless_pass_by_value",
    "needless_pass_by_ref_mut",
    "match_wildcard_for_single_variants",
    "unused_async",
)
