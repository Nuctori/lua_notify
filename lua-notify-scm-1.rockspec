package = "lua-notify"
version = "scm-1"
source = {
  url = "git+https://github.com/Nuctori/lua_notify",
}
description = {
  summary = "Lua bindings for notify (cross-platform file system notifications)",
  detailed = "Lua C extension exposing the notify Rust library: watch files and " ..
             "directories (recursive or not) on Linux (inotify), macOS " ..
             "(FSEvents) and Windows (ReadDirectoryChangesW). Events are " ..
             "accumulated in a queue and pulled with watcher:poll() / " ..
             "poll_wait() — Lua-safe (no cross-thread Lua calls).",
  license = "MIT",
  homepage = "https://github.com/Nuctori/lua_notify",
}
dependencies = {
  "lua >= 5.4",
}
build = {
  type = "make",
  build_target = "build",
  install_target = "install",
  build_variables = {
    LUA_VERSION = "$(LUA_VERSION)",
    PREFIX = "$(PREFIX)",
  },
}
