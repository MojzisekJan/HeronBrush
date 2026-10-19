# file_store

The files the editor reads and writes: maps, preferences, recent and favourite lists,
and the data a package ships. On the desktop these are plain file system calls; in the
browser the files live in memory, are put in place by the page before the editor starts
and are written back through a hook.

Part of [Heron Brush](../../README.md).
