//! Runs the Lua test suite (tests/test.lua) from within `cargo test`, so
//! mutation testing covers the full Lua assertion set.

use std::ffi::CStr;

use libloading::Library;
use mlua::ffi;

#[test]
fn lua_suite() {
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"));
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let module_file = if cfg!(target_os = "windows") {
        "lua_notify.dll"
    } else if cfg!(target_os = "macos") {
        "liblua_notify.dylib"
    } else {
        "liblua_notify.so"
    };
    let module_path = target_dir.join(profile).join(module_file);
    // Always rebuild the cdylib for the current source: cargo test builds the
    // rlib but a stale module would silently run old code. --profile test
    // reuses the test-profile dependency cache.
    let mut cmd = std::process::Command::new("cargo");
    if cfg!(target_os = "windows") {
        cmd.env("RUSTUP_TOOLCHAIN", "stable-x86_64-pc-windows-gnu");
    }
    let output = cmd
        .arg("build")
        .arg("--profile")
        .arg("test")
        .arg("--quiet")
        .output()
        .expect("run cargo build for the cdylib");
    assert!(
        output.status.success(),
        "cargo build for the cdylib failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        module_path.exists(),
        "module not built: {}",
        module_path.display()
    );

    unsafe {
        let state = ffi::luaL_newstate();
        assert!(!state.is_null(), "luaL_newstate failed");
        ffi::luaL_openlibs(state);

        let lib = Library::new(&module_path).expect("dlopen lua_notify module");
        let luaopen: libloading::Symbol<unsafe extern "C" fn(*mut ffi::lua_State) -> i32> = lib
            .get(b"luaopen_lua_notify")
            .expect("luaopen_lua_notify symbol");
        luaopen(state);
        ffi::lua_getglobal(state, c"package".as_ptr());
        ffi::lua_getfield(state, -1, c"loaded".as_ptr());
        ffi::lua_pushvalue(state, -3);
        ffi::lua_setfield(state, -2, c"lua_notify".as_ptr());
        ffi::lua_settop(state, 0);

        let script = c"tests/test.lua";
        if ffi::luaL_loadfile(state, script.as_ptr()) != 0 {
            panic!("cannot load tests/test.lua: {}", lua_error(state));
        }
        if ffi::lua_pcall(state, 0, 0, 0) != 0 {
            panic!("Lua test suite failed: {}", lua_error(state));
        }
        ffi::lua_close(state);
    }
}

unsafe fn lua_error(state: *mut ffi::lua_State) -> String {
    let msg = ffi::lua_tostring(state, -1);
    if msg.is_null() {
        "unknown error".to_string()
    } else {
        CStr::from_ptr(msg).to_string_lossy().into_owned()
    }
}
