# SQLite Vendored Sources

This directory contains vendored SQLite tool sources needed to build the
syntaqlite-codegen library. These files are copied from the main SQLite
source tree to make the crate self-contained for publishing to crates.io.

## Files

- `lemon.c` - Lemon parser generator (from SQLite's tool/lemon.c)
- `lempar.c` - Lemon parser template (from SQLite's tool/lempar.c)
- `mkkeywordhash.c` - Keyword hash generator (from SQLite's tool/mkkeywordhash.c)

## Updating

To update these files after upgrading SQLite:

```bash
tools/dev/vendor-sqlite-tools
```

## License

These files are part of SQLite and are in the public domain.
See https://www.sqlite.org/copyright.html
