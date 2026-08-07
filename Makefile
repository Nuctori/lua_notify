# lua_notify — Lua bindings for notify (cross-platform file system notifications).
#
# Build/test conventions follow the lua family: portable Makefile, `make test`
# runs the Lua suite. Cargo does the heavy lifting; the module is exposed as
# `target/release/lua_notify.{dll,so}` (Lua's require name).
#
# On Windows the module links the MinGW-built system Lua, so the GNU Rust
# toolchain is required (`rustup toolchain install stable-x86_64-pc-windows-gnu`).

LUA_BIN ?= lua5.4
CARGO ?= cargo

UNAME_S := $(shell uname -s 2>/dev/null)
ifeq ($(OS),Windows_NT)
CARGO_TOOLCHAIN := +stable-x86_64-pc-windows-gnu
CARGO_ARTIFACT := target/release/lua_notify.dll
MODULE := target/release/lua_notify.dll
else ifneq (,$(findstring MINGW,$(UNAME_S)))
CARGO_TOOLCHAIN := +stable-x86_64-pc-windows-gnu
CARGO_ARTIFACT := target/release/lua_notify.dll
MODULE := target/release/lua_notify.dll
else ifneq (,$(findstring MSYS,$(UNAME_S)))
CARGO_TOOLCHAIN := +stable-x86_64-pc-windows-gnu
CARGO_ARTIFACT := target/release/lua_notify.dll
MODULE := target/release/lua_notify.dll
else ifneq (,$(findstring Darwin,$(UNAME_S)))
CARGO_TOOLCHAIN :=
CARGO_ARTIFACT := target/release/liblua_notify.dylib
MODULE := target/release/lua_notify.so
else
CARGO_TOOLCHAIN :=
CARGO_ARTIFACT := target/release/liblua_notify.so
MODULE := target/release/lua_notify.so
endif

.PHONY: all build test test-rust mutants bench install clean

all: build

build: $(MODULE)

$(MODULE): src/lib.rs build.rs Cargo.toml Cargo.lock
	$(CARGO) $(CARGO_TOOLCHAIN) build --release
	@if [ "$(MODULE)" != "$(CARGO_ARTIFACT)" ]; then \
	  ln -sf $(notdir $(CARGO_ARTIFACT)) $(MODULE); \
	fi

test: build
	$(LUA_BIN) -e 'local s = package.config:sub(3,3); package.cpath = "target/release/?.dll" .. s .. "target/release/?.so" .. s .. package.cpath' tests/test.lua

test-rust: build
	$(CARGO) $(CARGO_TOOLCHAIN) test

# Mutation testing: injects code mutants and re-runs the whole Lua suite
# through tests/lua_tests.rs. Surviving mutants are real coverage gaps.
# Install once with: cargo install cargo-mutants
mutants:
	$(CARGO) $(CARGO_TOOLCHAIN) mutants --file src/lib.rs --no-shuffle --jobs 1

# Performance: native notify baseline vs the Lua binding (bridge overhead).
# Events are OS-bound; ratio measures the Lua-side queue/table cost.
BENCH_MAX_RATIO ?= 500
bench: build
	$(CARGO) $(CARGO_TOOLCHAIN) run --release --example bench_native \
	  | grep -E '^(poll_ns)=' > /tmp/notify_bench_native.txt
	$(LUA_BIN) -e 'local s = package.config:sub(3,3); package.cpath = "target/release/?.dll" .. s .. "target/release/?.so" .. s .. package.cpath' \
	  tests/bench.lua > /tmp/notify_bench_lua.txt
	@echo "=== lua_notify vs native notify (per-event) ==="
	@awk -F= 'FNR==NR { n[$$1]=$$2; next } $$1 ~ /_ns$$/ && $$1 in n { if (n[$$1] == 0) n[$$1] = 1; \
	  printf "%-12s native=%-8s lua=%-8s ratio=%.1fx\n", $$1, n[$$1], $$2, $$2/n[$$1]; \
	  if ($$2/n[$$1] > $(BENCH_MAX_RATIO)) { \
	    printf "FAIL: %s ratio %.1fx exceeds $(BENCH_MAX_RATIO)x\n", $$1, $$2/n[$$1] > "/dev/stderr"; exit 1 } }' \
	  /tmp/notify_bench_native.txt /tmp/notify_bench_lua.txt
	@echo "bench ok (max per-op ratio $(BENCH_MAX_RATIO)x)"

# Used by LuaRocks' make build type (see lua-notify-scm-1.rockspec).
PREFIX ?= /usr/local
INSTALL_LIBDIR ?= $(PREFIX)/lib/lua/$(LUA_VERSION)

install: build
	mkdir -p $(DESTDIR)$(INSTALL_LIBDIR)
	cp $(MODULE) $(DESTDIR)$(INSTALL_LIBDIR)/

clean:
	$(CARGO) $(CARGO_TOOLCHAIN) clean
