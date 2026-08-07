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
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::{channel, Receiver};

struct LuaWatcher {
    watcher: RecommendedWatcher,
    rx: Receiver<Event>,
}

impl mlua::UserData for LuaWatcher {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut(
            "watch",
            |_, this, (path, recursive): (String, Option<bool>)| {
                let mode = if recursive.unwrap_or(false) {
                    RecursiveMode::Recursive
                } else {
                    RecursiveMode::NonRecursive
                };
                this.watcher
                    .watch(std::path::Path::new(&path), mode)
                    .map_err(|e| mlua::Error::runtime(format!("lua_notify: watch failed: {e}")))
            },
        );

        methods.add_method_mut("unwatch", |_, this, path: String| {
            this.watcher
                .unwatch(std::path::Path::new(&path))
                .map_err(|e| mlua::Error::runtime(format!("lua_notify: unwatch failed: {e}")))
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
            let dur = std::time::Duration::try_from_secs_f64(sec.max(0.0))
                .map_err(|_| mlua::Error::runtime("lua_notify: invalid poll_wait seconds"))?;
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
///
/// 按子树拆分为浅层辅助函数（见下方 `*_kind_str`），保持每个 match 的
/// 认知复杂度在阈值以下；映射表由 `#[cfg(test)]` 单测锁定，重构不得
/// 改变任何已发布字符串。
fn event_kind_str(kind: &EventKind) -> &'static str {
    use notify::event::*;
    match kind {
        EventKind::Any => "any",
        EventKind::Access(k) => access_kind_str(k),
        EventKind::Create(k) => create_kind_str(k),
        EventKind::Modify(k) => modify_kind_str(k),
        EventKind::Remove(k) => remove_kind_str(k),
        EventKind::Other => "other",
    }
}

fn access_kind_str(k: &notify::event::AccessKind) -> &'static str {
    use notify::event::AccessKind::*;
    match k {
        Any => "access",
        Read => "access/read",
        Open(_) => "access/open",
        Close(_) => "access/close",
        Other => "access/other",
    }
}

fn create_kind_str(k: &notify::event::CreateKind) -> &'static str {
    use notify::event::CreateKind::*;
    match k {
        Any => "create",
        File => "create/file",
        Folder => "create/folder",
        Other => "create/other",
    }
}

fn modify_kind_str(k: &notify::event::ModifyKind) -> &'static str {
    use notify::event::ModifyKind::*;
    match k {
        Any => "modify",
        Data(d) => data_change_str(d),
        Metadata(m) => metadata_kind_str(m),
        Name(r) => rename_mode_str(r),
        Other => "modify/other",
    }
}

fn data_change_str(d: &notify::event::DataChange) -> &'static str {
    use notify::event::DataChange::*;
    match d {
        Any => "modify/data",
        Size => "modify/data/size",
        Content => "modify/data/content",
        Other => "modify/data/other",
    }
}

fn metadata_kind_str(m: &notify::event::MetadataKind) -> &'static str {
    use notify::event::MetadataKind::*;
    match m {
        Any => "modify/metadata",
        AccessTime => "modify/metadata/access_time",
        WriteTime => "modify/metadata/write_time",
        Permissions => "modify/metadata/permissions",
        Ownership => "modify/metadata/ownership",
        Extended => "modify/metadata/extended",
        Other => "modify/metadata/other",
    }
}

fn rename_mode_str(r: &notify::event::RenameMode) -> &'static str {
    use notify::event::RenameMode::*;
    match r {
        Any => "modify/rename",
        To => "modify/rename/to",
        From => "modify/rename/from",
        Both => "modify/rename/both",
        Other => "modify/rename/other",
    }
}

fn remove_kind_str(k: &notify::event::RemoveKind) -> &'static str {
    use notify::event::RemoveKind::*;
    match k {
        Any => "remove",
        File => "remove/file",
        Folder => "remove/folder",
        Other => "remove/other",
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

// ---------------------------------------------------------------------------
// unit tests: kind-string mapping is a stable public API (README documents
// the strings), so the full table is locked here and must not change.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::event_kind_str;
    use notify::event::*;

    #[test]
    fn kind_strings_match_documented_api() {
        let cases: Vec<(EventKind, &'static str)> = vec![
            (EventKind::Any, "any"),
            (EventKind::Other, "other"),
            // access
            (EventKind::Access(AccessKind::Any), "access"),
            (EventKind::Access(AccessKind::Read), "access/read"),
            (
                EventKind::Access(AccessKind::Open(AccessMode::Read)),
                "access/open",
            ),
            (
                EventKind::Access(AccessKind::Close(AccessMode::Write)),
                "access/close",
            ),
            (EventKind::Access(AccessKind::Other), "access/other"),
            // create
            (EventKind::Create(CreateKind::Any), "create"),
            (EventKind::Create(CreateKind::File), "create/file"),
            (EventKind::Create(CreateKind::Folder), "create/folder"),
            (EventKind::Create(CreateKind::Other), "create/other"),
            // modify
            (EventKind::Modify(ModifyKind::Any), "modify"),
            (
                EventKind::Modify(ModifyKind::Data(DataChange::Any)),
                "modify/data",
            ),
            (
                EventKind::Modify(ModifyKind::Data(DataChange::Size)),
                "modify/data/size",
            ),
            (
                EventKind::Modify(ModifyKind::Data(DataChange::Content)),
                "modify/data/content",
            ),
            (
                EventKind::Modify(ModifyKind::Data(DataChange::Other)),
                "modify/data/other",
            ),
            (
                EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any)),
                "modify/metadata",
            ),
            (
                EventKind::Modify(ModifyKind::Metadata(MetadataKind::AccessTime)),
                "modify/metadata/access_time",
            ),
            (
                EventKind::Modify(ModifyKind::Metadata(MetadataKind::WriteTime)),
                "modify/metadata/write_time",
            ),
            (
                EventKind::Modify(ModifyKind::Metadata(MetadataKind::Permissions)),
                "modify/metadata/permissions",
            ),
            (
                EventKind::Modify(ModifyKind::Metadata(MetadataKind::Ownership)),
                "modify/metadata/ownership",
            ),
            (
                EventKind::Modify(ModifyKind::Metadata(MetadataKind::Extended)),
                "modify/metadata/extended",
            ),
            (
                EventKind::Modify(ModifyKind::Metadata(MetadataKind::Other)),
                "modify/metadata/other",
            ),
            (
                EventKind::Modify(ModifyKind::Name(RenameMode::Any)),
                "modify/rename",
            ),
            (
                EventKind::Modify(ModifyKind::Name(RenameMode::To)),
                "modify/rename/to",
            ),
            (
                EventKind::Modify(ModifyKind::Name(RenameMode::From)),
                "modify/rename/from",
            ),
            (
                EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
                "modify/rename/both",
            ),
            (
                EventKind::Modify(ModifyKind::Name(RenameMode::Other)),
                "modify/rename/other",
            ),
            (EventKind::Modify(ModifyKind::Other), "modify/other"),
            // remove
            (EventKind::Remove(RemoveKind::Any), "remove"),
            (EventKind::Remove(RemoveKind::File), "remove/file"),
            (EventKind::Remove(RemoveKind::Folder), "remove/folder"),
            (EventKind::Remove(RemoveKind::Other), "remove/other"),
        ];

        for (kind, expected) in cases {
            assert_eq!(
                event_kind_str(&kind),
                expected,
                "event_kind_str({kind:?}) must stay \"{expected}\""
            );
        }
    }
}
