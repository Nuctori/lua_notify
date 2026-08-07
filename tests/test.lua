-- lua_notify test suite. Run with `make test`.
--
-- Uses a real temporary directory: create/modify/rename/delete files and
-- assert the polled events carry the expected kind strings (real_user entry).

local function assert_eq(actual, expected, msg)
  if actual ~= expected then
    error((msg or "assert_eq failed") .. string.format(": expected %s, got %s",
          tostring(expected), tostring(actual)), 2)
  end
end

local function assert_true(cond, msg)
  if not cond then error(msg or "assert_true failed", 2) end
end

local notify = require("lua_notify")
assert_eq(type(notify), "table", "module must load")

-- temp dir (real filesystem, platform-independent)
local dir = os.tmpname() .. ".d"
os.remove(dir)
os.execute('mkdir "' .. dir .. '"')
-- normalize: Lua on Windows sees backslashes in os.tmpname; use forward
dir = dir:gsub("\\", "/")

local function kinds_since(w, wait)
  os.execute("sleep " .. tostring(wait or 0.6))
  local out = {}
  local events = w:poll()
  if events then
    for _, ev in ipairs(events) do out[#out + 1] = ev.kind end
  end
  return out
end

local function contains(t, v)
  for _, x in ipairs(t) do if x == v then return true end end
  return false
end

-- ---------------------------------------------------------------------------
-- watch + create / modify / rename / remove
-- ---------------------------------------------------------------------------

local w = notify.new()
w:watch(dir, true)

local f = io.open(dir .. "/a.txt", "w")
f:write("hello")
f:close()

os.rename(dir .. "/a.txt", dir .. "/b.txt")
os.remove(dir .. "/b.txt")

local kinds = kinds_since(w)
assert_true(contains(kinds, "create"), "create event kind; got " .. table.concat(kinds, ","))
assert_true(contains(kinds, "modify"), "modify event kind")
assert_true(contains(kinds, "modify/rename/from"), "rename/from kind")
assert_true(contains(kinds, "modify/rename/to"), "rename/to kind")
assert_true(contains(kinds, "remove"), "remove event kind")

-- event rows carry kind + paths (array of strings, 1-based)
local events = w:poll()
assert_eq(events, nil, "poll returns nil when queue is empty")
local ev = w:poll_wait(0.2)
assert_eq(ev, nil, "poll_wait times out to nil")

-- ---------------------------------------------------------------------------
-- unwatch stops delivery
-- ---------------------------------------------------------------------------

w:unwatch(dir)
local f2 = io.open(dir .. "/c.txt", "w")
f2:close()
local after = kinds_since(w, 0.6)
assert_eq(#after, 0, "no events after unwatch; got " .. #after)

-- ---------------------------------------------------------------------------
-- non-recursive watch does not deliver into subdirectories
-- ---------------------------------------------------------------------------

local w2 = notify.new()
w2:watch(dir, false)
os.execute('mkdir "' .. dir .. '/sub" 2>nul')
local f3 = io.open(dir .. "/sub/x.txt", "w")
f3:close()
os.execute("sleep 0.5")
local sub_events = w2:poll()
if sub_events then
  local found = false
  for _, ev in ipairs(sub_events) do
    for _, p in ipairs(ev.paths) do
      if p:find("x%.txt") then found = true end
    end
  end
  assert_eq(found, false, "non-recursive watch should not see subdirectory file events")
end

-- recursive watch does see them
local w3 = notify.new()
w3:watch(dir, true)
os.execute('mkdir "' .. dir .. '/sub2" 2>nul')
local f4 = io.open(dir .. "/sub2/y.txt", "w")
f4:close()
os.execute("sleep 0.5")
local rec_events = w3:poll()
local found = false
if rec_events then
  for _, ev in ipairs(rec_events) do
    for _, p in ipairs(ev.paths) do
      if p:find("sub2") then found = true end
    end
  end
end
assert_true(found, "recursive watch should see subdirectory events")

-- ---------------------------------------------------------------------------
-- watch failure on nonexistent path
-- ---------------------------------------------------------------------------

local w4 = notify.new()
local ok, err = pcall(function() w4:watch(dir .. "/does-not-exist-zzz", true) end)
assert_eq(ok, false, "watch on nonexistent path must error")
assert_true(tostring(err):find("lua_notify") ~= nil, "error carries lua_notify prefix")

-- cleanup
os.execute('rmdir /s /q "' .. dir .. '" 2>nul')

print("lua_notify tests passed")
