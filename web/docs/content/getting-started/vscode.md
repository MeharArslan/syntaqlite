+++
title = "VS Code"
description = "Install the extension — diagnostics, formatting, and completions out of the box."
weight = 1
+++

# VS Code

## Install the extension

Search for **syntaqlite** in the Extensions panel (`Cmd+Shift+X` /
`Ctrl+Shift+X`) and click Install. The extension bundles the syntaqlite binary
for your platform — no separate install needed.

## Try it out

Create a file called `demo.sql` and paste this in:

```sql
select id,name,email from users wehre active=1 order by name
```

You should immediately see `wehre` underlined in red — syntaqlite caught the
typo. Fix it to `where` and the error disappears.

## Format your SQL

Open the command palette (`Cmd+Shift+P` / `Ctrl+Shift+P`) and run **Format
Document**. The query becomes:

```sql
SELECT id, name, email
FROM users
WHERE active = 1
ORDER BY name;
```

Keywords are uppercased, clauses are broken onto separate lines, and a
semicolon is appended. To format automatically on every save, enable
`editor.formatOnSave` in your VS Code settings.

## Get completions

In your SQL file, type `SEL` and you'll see a completion popup offering
`SELECT`. After `FROM `, completions include SQL keywords like `WHERE` and
`ORDER`. If you've set up a schema (next step), you'll also see table and
column names.

## Add schema validation

Without a schema, syntaqlite validates against an empty catalog — syntax errors
and built-in function checks work, but unknown tables and columns won't be
caught. To enable full validation, set up a schema file — see the
[schema validation guide](@/guides/schema-validation.md) for instructions.

Once configured, go back to `demo.sql` and change `name` to `nme`:

```sql
SELECT id, nme, email
FROM users
WHERE active = 1
ORDER BY name;
```

You'll see a warning on `nme` with a suggestion: *did you mean 'name'?*

See the [config file reference](@/reference/config-file.md) for glob-based
schema routing, formatting options, and the full `syntaqlite.toml` format.
