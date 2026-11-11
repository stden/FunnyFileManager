//! Purpose: Every tool handed the server validates its path before anything else.

use super::*;

/// Names of the methods called in a syntax node.
#[derive(Default)]
pub struct MethodCalls(pub Vec<String>);

impl<'ast> Visit<'ast> for MethodCalls {
    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        self.0.push(call.method.to_string());
        visit::visit_expr_method_call(self, call);
    }
}

fn takes_server(signature: &Signature) -> bool {
    let mut parameter_types = Idents::default();
    for input in &signature.inputs {
        if let FnArg::Typed(typed) = input {
            parameter_types.visit_type(&typed.ty);
        }
    }
    parameter_types.0.iter().any(|name| name == "FfmServer")
}

fn starts_by_validating(body: &Block) -> bool {
    let mut first_calls = MethodCalls::default();
    if let Some(first) = body.stmts.first() {
        first_calls.visit_stmt(first);
    }
    first_calls
        .0
        .iter()
        .any(|method| method.starts_with("safe_path"))
}

/// A top-level function handed `&FfmServer` starts by calling `safe_path*`.
pub fn unvalidated_tool_violations(sources: &[SourceFile]) -> Vec<String> {
    let mut messages = Vec::new();
    for file in sources
        .iter()
        .filter(|file| !file.is_test() && file.relative != SERVER)
    {
        for item in file.syntax().items {
            let Item::Fn(function) = item else { continue };
            if takes_server(&function.sig) && !starts_by_validating(&function.block) {
                let name = &function.sig.ident;
                let line = name.span().start().line;
                messages.push(format!(
                    "{}:{line}: `{name}` is handed the server but does not start with `server.safe_path*`",
                    file.relative
                ));
            }
        }
    }
    messages
}

#[test]
fn tool_rule_reports_validation_that_is_not_the_first_statement() {
    let sources = [
        SourceFile::new(
            "mcp/src/grep.rs",
            "pub(super) fn grep(server: &FfmServer, params: GrepParams) -> Result<(), McpError> {\n    let hits = scan(&params.path);\n    let path = server.safe_path(&params.path)?;\n    Ok(())\n}\n",
        ),
        SourceFile::new(
            "mcp/src/reports.rs",
            "pub(super) fn analyse(server: &FfmServer, params: Params) -> Result<(), McpError> {\n    let root = server.safe_path(&params.path)?;\n    Ok(())\n}\n",
        ),
    ];
    assert_eq!(
        unvalidated_tool_violations(&sources),
        ["mcp/src/grep.rs:1: `grep` is handed the server but does not start with `server.safe_path*`"]
    );
}
