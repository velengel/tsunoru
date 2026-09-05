# ADR 0037: Inspect the source database through byte snapshots

## context

Review R011 identifies that SQLite mode=ro may still write a source WAL shared-memory file.
[SQLite WAL documentation](https://www.sqlite.org/wal.html#read_only_databases) distinguishes read-only SQL access from auxiliary-file creation.
The verifier must not attach SQLite to the user's source database.

## decision

Perform source-database SQL inspection only on a disposable copy of a byte-stable database file set.

## rejected options

- Keep mode=ro on the source: auxiliary source files can still change.
- Copy only the main database: committed changes may still exist in WAL.
- Silently accept a changing source: file-by-file copies are not an online-backup guarantee.

## consequences

Read the main database and its WAL/SHM sidecars as bytes, require repeated reads to match, and recover/check only the copied database.
Compare the source file set again after verification without opening a source SQLite connection.
Concurrent source changes cause verification to fail; this workflow is for a quiescent source and is not a general live-backup tool.
Copied SHM can be rebuilt inside the disposable directory, and all snapshot files are removed with that directory.
