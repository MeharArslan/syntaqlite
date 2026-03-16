+++
title = "VS Code"
description = "Install the extension — diagnostics, formatting, and completions out of the box."
weight = 1
+++

# VS Code

## 1. Install

Install **[syntaqlite](https://marketplace.visualstudio.com/items?itemName=syntaqlite.syntaqlite)**
from the VS Code Marketplace, or search for `syntaqlite` in the Extensions
panel (`Cmd+Shift+X`). The extension bundles the binary — no other setup
needed.

## 2. See diagnostics

Create a file called `demo.sql` and paste this in:

```sql
select id,name,email from users wehre active=1 order by name
```

`wehre` is immediately underlined in red. Fix it to `where` and the error
clears.

## 3. Format

With the cursor in `demo.sql`, run **Format Document** (`Shift+Alt+F`). The
query becomes:

```sql
SELECT id, name, email
FROM users
WHERE active = 1
ORDER BY name;
```

To format automatically on every save, add this to your VS Code settings:

```json
{
  "editor.formatOnSave": true
}
```

## 4. Add a schema

So far syntaqlite only checks syntax. To catch unknown tables and columns,
create a file called `schema.sql` next to `demo.sql`:

```sql
CREATE TABLE users (id INTEGER, name TEXT, email TEXT, active INTEGER);
```

Then create `syntaqlite.toml` in the same directory:

```toml
[schemas]
"**/*.sql" = ["schema.sql"]
```

Go back to `demo.sql` and change `name` to `nme`:

```sql
SELECT id, nme, email
FROM users
WHERE active = 1
ORDER BY name;
```

You'll see `nme` underlined with a warning: *unknown column 'nme' — did you
mean 'name'?* Fix it back to `name` and the warning disappears.

## 5. Completions

Now that you have a schema, type a new query in `demo.sql`. After `FROM `,
you'll see `users` offered as a completion. After `SELECT `, you'll see `id`,
`name`, `email`, and `active`.

## Next steps

- [Formatting options](@/reference/formatting-options.md) — line width,
  keyword casing, semicolons
- [Config file reference](@/reference/config-file.md) — glob-based schema
  routing and all `syntaqlite.toml` options
- [Schema validation guide](@/guides/schema-validation.md) — multi-schema
  setups, strict mode, CI integration
