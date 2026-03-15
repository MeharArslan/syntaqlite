+++
title = "Formatting in CI"
description = "Enforce consistent SQL formatting in your CI pipeline."
weight = 1
+++

# Formatting in CI

Run syntaqlite in CI to enforce consistent SQL formatting across your team.

## Check mode

Use `--check` to verify that files are already formatted without modifying
them. It exits with code 1 if any file would change:

```bash
syntaqlite fmt --check "**/*.sql"
```

## GitHub Actions

```yaml
name: SQL lint
on: [pull_request]

jobs:
  format-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install syntaqlite
        run: pip install syntaqlite

      - name: Check formatting
        run: syntaqlite fmt --check "**/*.sql"
```

## Pre-push hook

Add a git hook to check SQL formatting before each push:

```bash
#!/bin/bash
# .git/hooks/pre-push

failed=0
for f in $(git diff --name-only origin/main --diff-filter=ACM | grep '\.sql$'); do
  if ! diff -q <(syntaqlite fmt "$f") "$f" > /dev/null 2>&1; then
    echo "Not formatted: $f"
    failed=1
  fi
done

exit $failed
```

Make it executable:

```bash
chmod +x .git/hooks/pre-push
```

## Formatting at scale

For large codebases, use glob patterns:

```bash
syntaqlite fmt -i "**/*.sql"
```

syntaqlite is fast — it reuses internal allocations across files, so formatting
thousands of files is practical.
