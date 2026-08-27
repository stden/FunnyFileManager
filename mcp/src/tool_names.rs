// The tool list, and nothing else.
//
// WHY its own file, and why plain `//` comments rather than a `//!` module
// header: the FunnyFileManager UI has to render this list too and cannot
// link against a binary crate, so it `include!`s this file verbatim — and an
// inner doc comment is a parse error anywhere but the top of a module.
// Anything added here must stand alone: no `use`, no `mod`, nothing that
// names the rest of the crate.

/// Every tool this server registers, in registration order — the single source
/// of truth for "what does this server expose".
///
/// WHY this exists rather than a comment: the `FunnyFileManager` UI used to ship
/// an "MCP status" screen with its own copy of this list, and the copy drifted —
/// it still advertised a tool that had been removed and a count that was three
/// short. That screen now `include!`s this file instead, so the doc comments
/// here are linted by both crates: keep every identifier in backticks.
/// `tool_names_match_the_registered_router` pins this list to the router the
/// server actually builds, so any tool added or deleted below fails the test
/// until the list is updated with it. Anything that needs to display the tool
/// set reads it from here.
pub const TOOL_NAMES: &[&str] = &[
    "list_directory",
    "sort_directory",
    "get_file_info",
    "view_file",
    "create_file",
    "write_file",
    "append_file",
    "create_directory",
    "copy",
    "move_rename",
    "delete",
    "search",
    "grep",
    "tree",
    "glob",
    "find",
    "replace",
    "delete_by_pattern",
    "list_drives",
    "disk_usage",
    "unique_name",
    "file_checksum",
    "format_size_tool",
    "batch_copy",
    "batch_move",
    "batch_delete",
    "create_symlink",
    "read_symlink",
    "permissions",
    "set_bookmark",
    "get_bookmark",
    "list_bookmarks",
    "delete_bookmark",
    "system_info",
    "s3_list_profiles",
    "s3_list_buckets",
    "s3_list_objects",
    "s3_download",
    "s3_upload",
    "s3_delete",
    "s3_copy",
];
