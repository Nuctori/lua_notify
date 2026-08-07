# lua_notify

[中文文档](README_zh.md) | English

Lua 5.4 bindings for [notify](https://github.com/notify-rs/notify) — a
cross-platform file system notification library. One API watches files and
directories on Linux (inotify), macOS (FSEvents) and Windows
(ReadDirectoryChangesW).

Fills the Lua ecosystem gap: existing watchers (linotify / lua-inotify) are
Linux-only; `lua_notify` is cross-platform with no external commands.

## Install

```sh
# from source
make build            # target/release/lua_notify.{dll,so}
make test             # run the Lua test suite
make install          # into LUA_VERSION lib dir

# via LuaRocks (requires a Rust toolchain + cargo)
luarocks make lua-notify-scm-1.rockspec
```

## API

```lua
local notify = require("lua_notify")

local w = notify.new()
w:watch("/path/to/dir", true)      -- recursive
w:watch("/path/to/file", false)    -- single file

-- drain pending events (non-blocking; nil when empty)
local events = w:poll()
if events then
  for _, ev in ipairs(events) do
    print(ev.kind)                 -- "create/file", "modify/data", ...
    for _, p in ipairs(ev.paths) do print(p) end
  end
end

-- block up to 0.5s for the next event
local ev = w:poll_wait(0.5)

w:unwatch("/path/to/dir")          -- stop watching
```

### Event kinds (stable strings)

| kind | meaning |
|------|---------|
| `create/file` `create/folder` | file/folder created |
| `modify/data/content` `modify/data/size` | content/size changed |
| `modify/metadata/*` | metadata changed (permissions, times, ...) |
| `modify/rename/to` `modify/rename/from` | rename destination / source |
| `remove/file` `remove/folder` | file/folder removed |
| `access/read` `access/open` `access/close` | read/open/close (platform-dependent) |
| `any` `other` `*/other` | catch-all kinds |

## Why poll (not callbacks)

Lua 5.4 has no event loop and its state is **not thread-safe**: notify's
background-thread callbacks must never call Lua. `lua_notify` accumulates
events in a queue inside the watcher and lets Lua pull with `poll()` /
`poll_wait()` on its own schedule — safe and fits Lua's synchronous model.

## Design notes

- **Module mode**: links the host Lua (no vendored VM), works in any Lua 5.4
  process via `require("lua_notify")`.
- **Pure Rust**: notify's native per-platform backends (inotify/FSEvents/
  ReadDirectoryChangesW), no external commands.
- **Errors**: watch on a nonexistent path raises `lua_notify: ...` runtime
  errors.
- **Event delivery latency**: FSEvents (macOS) can delay events by ~1s;
  inotify watch limits are governed by `fs.inotify.max_user_watches`.

## Tests & CI

- `tests/test.lua` — real-directory suite: create/modify/rename/remove kinds,
  poll/poll_wait semantics, unwatch stops delivery, recursive vs non-recursive,
  error paths (real user entry).
- `tests/lua_tests.rs` — runs the Lua suite inside `cargo test`.
- CI (GitHub Actions): build + Lua tests + Rust tests + bench on
  Linux / macOS / Windows. Releases build prebuilt artifacts per tag.

## License

MIT
