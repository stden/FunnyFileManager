<!-- Purpose: Setup and limits of the provider token usage panel. -->
# Token usage

Open **F9 → Commands → Token usage**. The report shows input, output,
cached input and request counts for the last 30 completed UTC days.
It refreshes automatically every 60 seconds while open; F5 refreshes now.
Arrows and PgUp/PgDn scroll; Esc or F10 closes the panel.

Provide credentials to the process that starts FFM:

| Provider   | Environment variable        | Required access                     |
| ---------- | --------------------------- | ----------------------------------- |
| OpenAI     | `OPENAI_ADMIN_KEY`          | API organization usage read access  |
| Anthropic  | `ANTHROPIC_ADMIN_KEY`       | Organization analytics access       |
| OpenRouter | `OPENROUTER_MANAGEMENT_KEY` | Management key for account activity |

Keys stay in memory and are never written to `ffm.yml`, logs or Git.
A missing credential appears as **Not configured**. Access denied, rate limits
and failed requests appear as errors, never as zero tokens.

OpenAI reports the completions usage endpoint; embeddings and unrelated products
are outside this report. Anthropic uses the Usage and Cost Analytics API's usage
endpoint. OpenRouter uses account activity. Consumer subscription limits and
remaining chat messages are not API token totals. Provider reports can be delayed.
Cached input is already included in input; OpenRouter does not report its cache
breakdown through this endpoint, so that cell is **n/a**.
There is no cross-provider total: traffic routed through OpenRouter could also
appear in a direct provider account, and adding both would double-count it.

HTTP requests use HTTPS, reject redirects, have time and response-size limits,
and follow pagination with a finite limit. Closing the report does not reopen
it when an old request completes. Credentials are not needed for the test suite;
tests use synthetic reports. Replayed and isolated sessions cannot read provider
keys or call these APIs.

Official endpoint references:

- [OpenAI completions usage](https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage/methods/completions).
- [Anthropic organization analytics](https://platform.claude.com/docs/en/manage-claude/usage-cost-analytics-api).
- [OpenRouter activity](https://openrouter.ai/docs/api/api-reference/analytics/get-user-activity-grouped-by-endpoint).
