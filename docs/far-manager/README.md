# Far Manager reference docs (upstream)

This project is inspired by Far Manager. To keep the repository small (and avoid vendoring third‑party content by default), upstream Far Manager documentation can be downloaded on demand into `docs/upstream/far-manager/` (ignored by git).

## Download

From the repo root:

```powershell
.\scripts\fetch_far_manager_docs.ps1
```

Outputs a `SOURCE.json` file with the upstream URL and commit SHA used.

## Notes

- If you want to commit the downloaded docs, remove `/docs/upstream/` from `.gitignore` first.
- Review upstream licensing/attribution requirements before redistributing documentation.
