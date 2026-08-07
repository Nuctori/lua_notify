-- lua_notify benchmark (bridge overhead). Output is `name=value` lines
-- consumed by `make bench` (see the awk comparison in the Makefile).

local notify = require("lua_notify")

local dir = os.tmpname() .. ".d"
os.remove(dir)
os.execute('mkdir "' .. dir .. '"')
dir = dir:gsub("\\", "/")

local w = notify.new()
w:watch(dir, false)

-- fire N create/delete cycles, then time poll() draining the queue
local N = 100
local f = dir .. "/f.txt"
local t0 = os.clock()
for i = 1, N do
  local h = io.open(f, "w"); h:write(i); h:close()
  os.remove(f)
end
os.execute("sleep 0.5")
local events = w:poll()
local got = events and #events or 0
local dt = os.clock() - t0

-- measure pure poll cost on empty queue
local t1 = os.clock()
for _ = 1, 1000 do w:poll() end
local poll_empty = (os.clock() - t1) / 1000 * 1e9

os.execute('rmdir /s /q "' .. dir .. '" 2>nul')

print(string.format("poll_ns=%.0f", (dt / N) * 1e9))
print(string.format("events_received=%d", got))
print(string.format("poll_empty_ns=%.0f", poll_empty))
