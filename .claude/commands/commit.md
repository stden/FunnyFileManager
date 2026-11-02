<!-- Purpose: Documents the Claude command for committing validated changes. -->

Stage all changes, create a commit, pull with rebase, and push to remote.

Steps:
1. Run `git status --short` to check for changes. If working tree is clean (no output), report "Nothing to commit" and stop.
2. Run `git add -u` to stage tracked file changes. Then review untracked files from `git status --short` — only stage files that belong in the repo. Never stage `.env`, credentials, or files over 10 MB.
3. Run `git diff --staged --stat` to review what will be committed.
4. Run `git log --oneline -5` to see recent commit message style.
5. Draft a concise commit message summarizing the staged changes. Prefix new functions/methods with `[+]`, removed functions/methods with `[-]`, and refactored functions/methods with `[!]` in the message body (e.g. `[+] my_new_function`, `[-] my_old_function`, `[!] my_refactored_function`).
6. Create the commit with the message.
7. Run `git pull --rebase`. If there are rebase conflicts, stop and report the conflicting files — do NOT force push or auto-resolve.
8. Run `git push` to push the commit.
9. Run `git status` to verify everything is clean.
