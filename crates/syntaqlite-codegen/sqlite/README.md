# SQLite Vendored Sources

This directory contains vendored SQLite tool sources and grammar files needed
to build syntaqlite-codegen. These files are copied from the main SQLite
source tree to make the crate self-contained for publishing to crates.io.

## Files

- `lemon.c` - Lemon parser generator (from SQLite's tool/lemon.c)
- `lempar.c` - Lemon parser template (from SQLite's tool/lempar.c)
- `mkkeywordhash.c` - Keyword hash generator (from SQLite's tool/mkkeywordhash.c)
- `parse.y` - SQLite grammar file (from SQLite's src/parse.y)

## Build Integration

The C sources (lemon.c, mkkeywordhash.c) are compiled into the syntaqlite-codegen
binary via build.rs and exposed through FFI as `lemon_main()` and
`mkkeywordhash_main()`.

The parse.y grammar file is used as the base for generating the syntaqlite parser.

## Updating

To update these files after upgrading SQLite:

```bash
tools/dev/vendor-sqlite-tools
```

## License

These files are part of SQLite and are in the public domain.
See https://www.sqlite.org/copyright.html
