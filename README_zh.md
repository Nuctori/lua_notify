# lua_notify

[English](README.md) | 中文文档

[notify](https://github.com/notify-rs/notify)（跨平台文件系统通知库）的 Lua 5.4 绑定。一套 API 在 Linux（inotify）、macOS（FSEvents）、Windows（ReadDirectoryChangesW）上监听文件/目录变化。

填补 Lua 生态空白：现有方案（linotify / lua-inotify）仅支持 Linux；`lua_notify` 跨平台、无外部命令依赖。

## 安装

```sh
# 源码编译
make build            # 产出 target/release/lua_notify.{dll,so}
make test             # 运行 Lua 测试套件
make install          # 安装到 LUA_VERSION 对应 lib 目录

# 或通过 LuaRocks（需要 Rust 工具链）
luarocks make lua-notify-scm-1.rockspec
```

## API

```lua
local notify = require("lua_notify")

local w = notify.new()
w:watch("/path/to/dir", true)      -- 递归
w:watch("/path/to/file", false)    -- 单文件

-- 非阻塞取事件（空队列返回 nil）
local events = w:poll()
if events then
  for _, ev in ipairs(events) do
    print(ev.kind)                 -- "create/file"、"modify/data" ...
    for _, p in ipairs(ev.paths) do print(p) end
  end
end

-- 阻塞至多 0.5 秒等下一个事件
local ev = w:poll_wait(0.5)

w:unwatch("/path/to/dir")          -- 停止监听
```

### 事件类型（稳定字符串）

| kind | 含义 |
|------|------|
| `create/file` `create/folder` | 文件/文件夹创建 |
| `modify/data/content` `modify/data/size` | 内容/大小变化 |
| `modify/metadata/*` | 元数据变化（权限/时间等） |
| `modify/rename/to` `modify/rename/from` | 重命名目标/来源 |
| `remove/file` `remove/folder` | 文件/文件夹删除 |
| `access/read` `access/open` `access/close` | 读/开/关（平台相关） |
| `any` `other` `*/other` | 兜底类型 |

## 为什么用 poll（而非回调）

Lua 5.4 没有事件循环，且 Lua state **非线程安全**——notify 后台线程的回调绝不能调 Lua。`lua_notify` 把事件累积在 watcher 内部的队列里，由 Lua 用 `poll()`/`poll_wait()` 按自己的节奏拉取——安全且符合 Lua 同步模型。

## 设计要点

- **模块模式**：链接宿主 Lua（不内嵌 VM），任意 Lua 5.4 进程 `require("lua_notify")` 可用。
- **纯 Rust**：notify 各平台原生后端（inotify/FSEvents/ReadDirectoryChangesW），无外部命令。
- **错误**：watch 不存在的路径抛 `lua_notify: ...` 运行时错误。
- **事件延迟**：macOS FSEvents 可能延迟 ~1s；inotify 监听上限受 `fs.inotify.max_user_watches` 约束。

## 测试与 CI

- `tests/test.lua` — 真实目录套件：create/modify/rename/remove 类型、poll/poll_wait 语义、unwatch 停止投递、递归 vs 非递归、错误路径（真实用户入口）。
- `tests/lua_tests.rs` — 在 `cargo test` 内跑 Lua 套件。
- CI（GitHub Actions）：Linux / macOS / Windows 三平台构建 + Lua 测试 + Rust 测试 + 基准；打 tag 自动出预编译产物。

## License

MIT
