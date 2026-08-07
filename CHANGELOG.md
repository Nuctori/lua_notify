# Changelog

## [0.1.0] - 2026-08-08

Initial release.

- `notify.new()` watcher userdata: `watch(path, recursive)` / `unwatch(path)`
- `poll()` non-blocking event drain (nil when empty) and `poll_wait(sec)`
  blocking receive — Lua-safe (no cross-thread Lua calls)
- EventKind flattened to stable strings: `create/file`, `modify/data`,
  `modify/rename/to|from`, `remove/file`, `access/*`, catch-alls
- Cross-platform: Linux inotify / macOS FSEvents / Windows ReadDirectoryChangesW
  (notify 8), pure Rust, module mode for host Lua 5.4
- Real-directory Lua test suite (create/modify/rename/remove, unwatch,
  recursive vs non-recursive, error paths), `cargo test` integration,
  CI on Linux/macOS/Windows, bench vs native notify
