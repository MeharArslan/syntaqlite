+++
title = "Formatting in CI"
description = "Enforce consistent SQL formatting in your CI pipeline."
weight = 1
+++

# Formatting in CI

Run syntaqlite in CI to enforce consistent SQL formatting across your team.

## Check mode

Use `syntaqlite fmt` without `-i` and compare the output to the original. If
they differ, the file isn't formatted. A simple approach:

```bash
#!/bin/bash
set -e

failed=0
for f in $(find . -name '*.sql'); do
  if ! diff -q <(syntaqlite fmt "$f") "$f" > /dev/null 2>&1; then
    echo "Not formatted: $f"
    failed=1
  fi
done

exit $failed
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
        run: |
          curl --proto '=https' --tlsv1.2 -LsSf \
            https://github.com/LalitMaganti/syntaqlite/releases/latest/download/syntaqlite-cli-installer.sh \
            | sh

      - name: Check formatting
        run: |
          for f in $(find . -name '*.sql'); do
            diff <(syntaqlite fmt "$f") "$f" || { echo "::error file=$f::Not formatted"; exit 1; }
          done
```

## Pre-commit hook

Add a git hook to format SQL before each commit:

```bash
#!/bin/bash
# .git/hooks/pre-commit

sql_files=$(git diff --cached --name-only --diff-filter=ACM | grep '\.sql$')
if [ -n "$sql_files" ]; then
  echo "$sql_files" | xargs syntaqlite fmt -i
  echo "$sql_files" | xargs git add
fi
```

Make it executable:

```bash
chmod +x .git/hooks/pre-commit
```

## Formatting at scale

For large codebases, use glob patterns:

```bash
syntaqlite fmt -i "**/*.sql"
```

syntaqlite is fast — it reuses internal allocations across files, so formatting
thousands of files is practical.
