"""The repository files each developer script runs from, for the tests that copy it.

A script imports its own modules from scripts/, so a sandbox checkout that
runs it needs every one of them: such a test copies the whole tuple.
"""

# ffm.py, the launcher modules it imports, and shell_path.py.
LAUNCHER = (
    "ffm.py",
    "scripts/launcher_build.py",
    "scripts/launcher_colorer.py",
    "scripts/launcher_install.py",
    "scripts/launcher_output.py",
    "scripts/launcher_paths.py",
    "scripts/launcher_process.py",
    "scripts/shell_path.py",
)
# lint.py, the step runner and the checks it imports, and shell_path.py.
LINT = ("scripts/lint.py", "scripts/lint_checks.py", "scripts/lint_steps.py", "scripts/shell_path.py")
# cargo_preflight.py, the module that reads its limit, and the lint.py files it runs through.
CARGO_PREFLIGHT = ("scripts/cargo_preflight.py", "scripts/preflight_limit.py", *LINT)
# k9s_eks.py, the modules it imports, and shell_path.py.
K9S_EKS = ("scripts/k9s_eks.py", "scripts/k9s_eks_aws.py", "scripts/k9s_eks_shell.py", "scripts/shell_path.py")
# install_mcp.py and the module it imports.
INSTALL_MCP = ("install_mcp.py", "scripts/install_mcp_configs.py")
# create_desktop_shortcut.py, the module it imports, and shell_path.py.
SHORTCUT = ("create_desktop_shortcut.py", "scripts/shell_path.py", "scripts/shortcut_files.py")
