# Jira Browser, HTTP MCP Services, and AI Assistant

FunnyFileManager includes three built-in productivity integrations accessible from **F9 → Commands**, **Alt+F1 / Alt+F2** (drive menu), or the `:` command line:

- **Jira Issues** (`:jira`) — browse stories, read markdown details, edit JQL, and export `{KEY}/` folders or atomic `jira_exports/{YYYY-MM-DD}/` snapshots.
- **MCP Services** (`:mcps`) — discover Streamable HTTP MCP servers configured for Claude Code, Kiro, Junie, or `ffm.yml`, list their tools, and run `tools/call`.
- **AI Assistant** (`:ai`, `:ask`) — Warp-style panel-aware chat backed by Anthropic Messages, OpenAI Chat Completions, or Codex Responses APIs.

## 1. Jira Issues (`:jira`)

### Configuration

Configure optional defaults in `ffm.yml`:

```yaml
jira:
  url: https://jira.example.test
  jql: assignee = currentUser() AND statusCategory != Done ORDER BY updated DESC
  export_dir: ./jira-stories
  tls_verify: true
  page_size: 50
```

Credentials are read strictly from environment variables (never stored in YAML):

| Variable                                                                                               | Purpose                             |
| ------------------------------------------------------------------------------------------------------ | ----------------------------------- |
| `JIRA_PERSONAL_TOKEN` (or `JIRA_PAT`, `JIRA_TOKEN`, `JIRA_API_TOKEN`, `ATLASSIAN_JIRA_PERSONAL_TOKEN`) | Personal Access Token (Bearer auth) |
| `JIRA_URL`                                                                                             | Base URL override                   |
| `JIRA_TLS_VERIFY`                                                                                      | `true` or `false`                   |
| `JIRA_CA_CERT`                                                                                         | Path to custom PEM CA bundle        |

### Commands & Keys

- `:jira` / `:jira jql <query>` / `:jira export [KEY]` / `:jira snapshot [query]` / `:jira edit`
- `Enter` / `F3`: open detail reader | `Space` / `Insert`: toggle mark | `F2` / `Ctrl+J`: edit JQL
- `F5`: export selected/marked `{KEY}/{desc.md,issue.md,comment.md,comment.id}` (`Shift+F5` overwrites existing files)
- `F6`: export all loaded issues (`Shift+F6` overwrites) | `F7` / `Ctrl+S`: atomic `jira_exports/{DATE}/` snapshot

## 2. Streamable HTTP MCP Services (`:mcps`)

FFM reads `"type": "http"` / `"streamable-http"` / `"streamableHttp"` entries from `~/.claude.json`, `.mcp.json`, `~/.kiro/settings/mcp.json`, `.kiro/settings/mcp.json`, `FFM_MCP_CONFIGS`, and `ffm.yml`:

```yaml
mcp_tls_verify: true
mcp_servers:
  docs:
    type: http
    url: https://mcp.example.test/docs
    headers:
      Authorization: "Bearer ${MCP_TOKEN}"
    disabledTools: []
```

Header `${VAR}` references expand from environment variables in memory only. Inside `:mcps`: `Tab` switches between Services and Tools, `Enter` / `F5` refreshes tools or edits JSON arguments and runs `tools/call`, and `F2` / `Ctrl+S` saves tool output to `mcp-output.txt`.

## 3. Warp-like AI Assistant (`:ai`, `:ask`)

Configure models/endpoints via environment variables (`ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_API_KEY`, `ANTHROPIC_DEFAULT_MODEL`, `OPENAI_API_KEY`, `OPENAI_BASE_URL`, `OPENAI_MODEL`, `CODEX_API_KEY`, `CODEX_BASE_URL`, `CODEX_MODEL`) or in `ffm.yml`:

```yaml
ai:
  default_provider: anthropic
  anthropic:
    base_url: https://api.example.test
    model: claude-haiku-5-5
  openai:
    base_url: https://api.example.test/v1
    model: gpt-4o
  codex:
    base_url: https://api.example.test/v1
    model: gpt-5.3-codex
```

Inside `:ai`: `Tab` / `Ctrl+P` cycles providers (`Alt+1/2/3` jumps directly), `F5` / `Ctrl+X` copies the suggested fenced shell command into the `:` prompt for manual confirmation (never auto-executed), and `F2` / `Ctrl+S` saves `ai-transcript.md`.
