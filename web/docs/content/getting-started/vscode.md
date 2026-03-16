+++
title = "VS Code"
description = "Install the extension — diagnostics, formatting, and completions out of the box."
weight = 1
+++

# VS Code

## Install

Search for **syntaqlite** in the Extensions panel (`Cmd+Shift+X` /
`Ctrl+Shift+X`) and click Install. No other setup needed — the extension
bundles the binary.

## Diagnostics

Create `demo.sql` and paste:

```sql
select id,name,email from users wehre active=1 order by name
```

`wehre` is immediately underlined — fix it to `where` and the error clears.

## Format

Run **Format Document** (`Shift+Alt+F`):

```sql
SELECT id, name, email
FROM users
WHERE active = 1
ORDER BY name;
```

Enable `editor.formatOnSave` to format automatically.

## Completions

Type `SEL` to see keyword completions. After `FROM `, you'll see table names
if you've configured a [schema](@/guides/schema-validation.md).

## Schema validation

Add a `syntaqlite.toml` to your project root to catch unknown tables and
columns with did-you-mean suggestions:

```toml
[schemas]
"**/*.sql" = ["schema.sql"]
```

See the [schema validation guide](@/guides/schema-validation.md) for setup
and the [config file reference](@/reference/config-file.md) for all options.

## Customize formatting

Override defaults in `syntaqlite.toml`:

```toml
[format]
line_width = 120
keyword_case = "lower"
```

See [formatting options](@/reference/formatting-options.md) for all settings.
