//! `lua_notify` — Lua bindings for [notify](https://github.com/notify-rs/notify),
//! a cross-platform file system notification library.
//!
//! Module mode: links the host Lua (never vendored), so the module can be
//! `require`d from any Lua 5.4 process.
//!
//! Poll model (why): Lua 5.4 has no event loop and its state is not
//! thread-safe, so notify's background-thread callbacks can never call Lua.
//! Events accumulate in an mpsc queue inside the watcher userdata; the Lua
//! side pulls them with `watcher:poll()` on its own schedule. This is safe
//! (no cross-thread Lua calls) and fits Lua's synchronous model.

use mlua::prelude::*;
use notify::{Event, EventKind, RecursiveMode, RecommendedWatcher, Watcher};
use std::sync::mpsc::{channel, Receiver};

struct LuaWatcher {
    watcher: RecommendedWatcher,
    rx: Receiver<Event>,
}

impl mlua::UserData for LuaWatcher {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("watch", |_, this, (path, recursive): (String, Option<bool>)| {
            let mode = if recursive.unwrap_or(false) {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            this.watcher
                .watch(std::path::Path::new(&path), mode)
                .map_err(|e| {
                    mlua::Error::runtime(format!("lua_notify: watch failed: {e}"))
                })
        });

        methods.add_method_mut("unwatch", |_, this, path: String| {
            this.watcher
                .unwatch(std::path::Path::new(&path))
                .map_err(|e| {
                    mlua::Error::runtime(format!("lua_notify: unwatch failed: {e}"))
                })
        });

        // poll() 非阻塞取事件：有事件返回数组，无事件返回 nil。
        methods.add_method("poll", |lua, this, ()| {
            let mut events = Vec::new();
            while let Ok(ev) = this.rx.try_recv() {
                events.push(event_row(lua, &ev)?);
            }
            if events.is_empty() {
                return Ok(mlua::Value::Nil);
            }
            let arr = lua.create_table_with_capacity(events.len(), 0)?;
            for (i, row) in events.into_iter().enumerate() {
                arr.set(i + 1, row)?;
            }
            Ok(mlua::Value::Table(arr))
        });

        // poll_wait(sec) 阻塞等待至多 sec 秒取一个事件（适合周期性轮询循环）。
        methods.add_method("poll_wait", |lua, this, sec: f64| {
            let dur = std::time::Duration::from_secs_f64(sec.max(0.0));
            match this.rx.recv_timeout(dur) {
                Ok(ev) => Ok(mlua::Value::Table(event_row(lua, &ev)?)),
                Err(_) => Ok(mlua::Value::Nil),
            }
        });
    }
}

/// 把 notify::Event 拍平成 Lua 表：kind 稳定字符串 + paths 数组。
fn event_row(lua: &Lua, ev: &Event) -> LuaResult<LuaTable> {
    let row = lua.create_table()?;
    row.set("kind", event_kind_str(&ev.kind))?;
    let paths = lua.create_table_with_capacity(ev.paths.len(), 0)?;
    for (i, p) in ev.paths.iter().enumerate() {
        paths.set(i + 1, p.to_string_lossy().into_owned())?;
    }
    row.set("paths", paths)?;
    Ok(row)
}

/// EventKind 树形枚举 → 稳定 Lua 字符串（如 "create/file"、"modify/data"）。
fn event_kind_str(kind: &EventKind) -> &'static str {
    use notify::event::*;
    match kind {
        EventKind::Any => "any",
        EventKind::Access(k) => match k {
            AccessKind::Any => "access",
            AccessKind::Read => "access/read",
            AccessKind::Open(_) => "access/open",
            AccessKind::Close(_) => "access/close",
            AccessKind::Other => "access/other",
        },
        EventKind::Create(k) => match k {
            CreateKind::Any => "create",
            CreateKind::File => "create/file",
            CreateKind::Folder => "create/folder",
            CreateKind::Other => "create/other",
        },
        EventKind::Modify(k) => match k {
            ModifyKind::Any => "modify",
            ModifyKind::Data(d) => match d {
                DataChange::Any => "modify/data",
                DataChange::Size => "modify/data/size",
                DataChange::Content => "modify/data/content",
                DataChange::Other => "modify/data/other",
            },
            ModifyKind::Metadata(m) => match m {
                MetadataKind::Any => "modify/metadata",
                MetadataKind::AccessTime => "modify/metadata/access_time",
                MetadataKind::WriteTime => "modify/metadata/write_time",
                MetadataKind::Permissions => "modify/metadata/permissions",
                MetadataKind::Ownership => "modify/metadata/ownership",
                MetadataKind::Extended => "modify/metadata/extended",
                MetadataKind::Other => "modify/metadata/other",
            },
            ModifyKind::Name(r) => match r {
                RenameMode::Any => "modify/rename",
                RenameMode::To => "modify/rename/to",
                RenameMode::From => "modify/rename/from",
                RenameMode::Both => "modify/rename/both",
                RenameMode::Other => "modify/rename/other",
            },
            ModifyKind::Other => "modify/other",
        },
        EventKind::Remove(k) => match k {
            RemoveKind::Any => "remove",
            RemoveKind::File => "remove/file",
            RemoveKind::Folder => "remove/folder",
            RemoveKind::Other => "remove/other",
        },
        EventKind::Other => "other",
    }
}

// ---------------------------------------------------------------------------
// module entry
// ---------------------------------------------------------------------------

#[mlua::lua_module]
fn lua_notify(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;

    // new() 创建 watcher userdata（后台线程由 notify 管理，事件进 mpsc 队列）。
    let new = lua.create_function(|lua, ()| {
        let (tx, rx) = channel::<Event>();
        let watcher = notify::recommended_watcher(move |res| {
            if let Ok(ev) = res {
                let _ = tx.send(ev);
            }
        })
        .map_err(|e| mlua::Error::runtime(format!("lua_notify: watcher init failed: {e}")))?;
        lua.create_userdata(LuaWatcher { watcher, rx })
    })?;
    exports.set("new", new)?;

    Ok(exports)
}
