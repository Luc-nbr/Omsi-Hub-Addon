-- Omsi-Hub's plugin: the phone and tablet companion (docs/COMPANION.md) - what the Omsi-Hub
-- Companion app on the driver's phone shows of the game, and what it may do in the bus.
--
-- Omsi-Hub's server (crates/hub-companion) runs beside the game while the player has the
-- companion on; it writes `data/companion.json` then, and this module talks to it only while
-- that file is there:
--   UDP 127.0.0.1:<port>   plugin -> Omsi-Hub: the game twice a second, as small JSON datagrams
--                          (`omsi.send`, at most 8 KB each: a long list goes in pages)
--   data/companion.json    Omsi-Hub -> plugin: the commands from the phone, numbered; each done
--                          once and acked by datagram (and the place whose street is asked for)
--
-- The phone presses only keys of the bus's IBIS, its ticket printer and its change dispenser
-- (the triggers this module found on the bus and told Omsi-Hub of); anything else is refused.

local json = require("hubjson")

local M = {}

M.FILE = "companion.json"
M.PORT = 47812
-- (a datagram's text at most: the API takes 8 KB)
local MAX = 7600

-- the words that make a trigger the IBIS's, the ticket printer's or the change dispenser's
local CHANGE = { "change", "wechsel", "muenz", "münz", "coin", "cash", "kasse", "geld" }
local PRINTER = { "ticket", "almex", "drucker", "fahrschein", "printer" }
local IBIS = { "ibis", "matrix", "ibox" }
-- the IBIS's variables read (where the bus has them) and its strings
local IBIS_VARS = {
  "IBIS_mode", "IBIS_Linie", "IBIS_LinieKurs", "IBIS_Linie_Suffix", "IBIS_Linie_Complex", "IBIS_Kurs",
  "IBIS_Route", "IBIS_RouteIndex", "IBIS_TerminusCode", "IBIS_TerminusIndex", "IBIS_busstop",
  "ibox_routenindex", "ibox_busstop",
}

local hook = {}
local on = false
local port = M.PORT
-- the session of Omsi-Hub's queue, and the last command done of it
local sid, done = nil, 0
local keys = { ibis = {}, printer = {}, change = {} }
local allowed = {}
local keys_for = nil
-- each list's version, and its text when it was last sent (a new version when it changes)
local lists = {}
-- the delay the bus had at each stop it left (by its number), for the trip under way
local left_at = {}
local left_trip = nil
local probe = nil
local wait = { live = 0, lists = 0, look = 0, keys = 0 }

local function fn(name)
  local t = omsi
  for part in name:gmatch("[^%.]+") do
    if type(t) ~= "table" then return nil end
    t = t[part]
  end
  return type(t) == "function" and t or nil
end

local function num(v)
  v = tonumber(v)
  if v == nil or v ~= v or v == math.huge or v == -math.huge then return nil end
  return v
end

-- `hooks`: signon and messages (the modules), accept() (the duty order signed), drive_free()
-- (the driver drives without the duty) and issued() of main.lua
function M.setup(hooks)
  hook = hooks or {}
end

local function send(t)
  if not (on and fn("send")) then return false end
  local text = json.encode(t)
  if #text > 8000 then return false end
  return omsi.send(port, text) == true
end

-- a list in pages of at most MAX characters each, under its own version
local function send_list(kind, key, items)
  local text = json.encode(items)
  local l = lists[kind] or { v = 0, text = nil }
  if l.text ~= text then
    l.v, l.text = l.v + 1, text
    lists[kind] = l
  end
  local pages, page, size = {}, {}, 0
  for _, it in ipairs(items) do
    local n = #json.encode(it) + 1
    if #page > 0 and size + n > MAX then
      pages[#pages + 1], page, size = page, {}, 0
    end
    page[#page + 1] = it
    size = size + n
  end
  if #page > 0 or #pages == 0 then pages[#pages + 1] = page end
  for i, p in ipairs(pages) do
    local t = { t = kind, v = l.v, page = i, pages = #pages }
    t[key] = p
    send(t)
  end
end

local function matches(name, words)
  local low = name:lower()
  for _, w in ipairs(words) do
    if low:find(w, 1, true) then return true end
  end
  return false
end

-- the bus's keys of its IBIS, ticket printer and change dispenser
local function find_keys()
  keys = { ibis = {}, printer = {}, change = {} }
  allowed = {}
  local list = fn("bus.triggers") and omsi.bus.triggers() or {}
  for _, name in ipairs(list) do
    if type(name) == "string" and not name:match("_off$") and not name:match("_drag$") and not name:match("^ai_") then
      local kind = matches(name, CHANGE) and "change" or matches(name, PRINTER) and "printer" or matches(name, IBIS) and "ibis" or nil
      if kind and #keys[kind] < 120 then
        keys[kind][#keys[kind] + 1] = name
        allowed[name] = true
      end
    end
  end
  for _, k in pairs(keys) do table.sort(k) end
end

local function tickets()
  local out = {}
  for i, t in ipairs(fn("bus.tickets") and omsi.bus.tickets() or {}) do
    if i > 40 then break end
    out[#out + 1] = { i = i, name = t.name or "", price = num(t.price) or 0, day = t.day_ticket == true }
  end
  return out
end

local function send_keys()
  local file = fn("bus.file") and omsi.bus.file() or ""
  if keys_for ~= file then
    keys_for = file
    find_keys()
  end
  send({ t = "keys", bus = file, ibis = keys.ibis, printer = keys.printer, change = keys.change, tickets = tickets() })
end

-- the trip under way: the game's duty, its stops (the delay at each one left) and the next one
local function duty()
  local d = fn("duty.get") and omsi.duty.get() or nil
  if type(d) ~= "table" then return nil, {} end
  local key = tostring(d.line) .. "/" .. tostring(d.tour) .. "/" .. tostring(d.trip)
  if key ~= left_trip then left_at, left_trip = {}, key end
  local stops = {}
  local next_index = nil
  local next_number = d.next_stop and tonumber(d.next_stop.number) or nil
  for k, s in ipairs(fn("duty.stops") and omsi.duty.stops() or {}) do
    local n = tonumber(s.number) or k
    stops[#stops + 1] = {
      name = s.name or "", arrival = num(s.arrival or s.arr), departure = num(s.departure or s.dep),
      calls = s.stops ~= false, passed = s.passed == true, x = num(s.x), y = num(s.y), delay = left_at[n],
    }
    if next_index == nil and ((next_number and n == next_number) or (not next_number and s.passed ~= true)) then next_index = k - 1 end
  end
  local out = {
    line = d.line or "", tour = d.tour or "", trip = math.tointeger(d.trip) or 0, trips = math.tointeger(d.trips) or 0,
    trip_name = d.trip_name or "", terminus = d.terminus or "", delay = num(d.delay) or 0,
    departure = num(d.departure), arrival = num(d.arrival),
    at_stop = d.at_stop == true, trip_done = d.trip_done == true,
    next = next_index or #stops, next_name = d.next_stop and d.next_stop.name or "",
  }
  local all = fn("duty.trips") and omsi.duty.trips() or {}
  local t = all[out.trip]
  if type(t) == "table" then
    out.number = t.line or ""
    if out.terminus == "" then out.terminus = t.terminus or "" end
    out.departure, out.arrival = out.departure or num(t.departure), out.arrival or num(t.arrival)
    if out.trip_name == "" then out.trip_name = t.name or "" end
  end
  return out, stops
end

local function bus()
  if fn("has_vehicle") and not omsi.has_vehicle() then return nil end
  local b = { speed = num(fn("speed") and omsi.speed()) or 0 }
  if fn("position") then
    local x, y, _, h = omsi.position()
    b.x, b.y, b.heading = num(x), num(y), num(h)
  end
  if fn("vehicle") then b.name = omsi.vehicle() or "" end
  if fn("bus.number") then b.number = omsi.bus.number() or "" end
  if fn("bus.passengers") then b.passengers = num(omsi.bus.passengers()) end
  if fn("bus.doors_open") then b.doors_open = omsi.bus.doors_open() end
  if fn("bus.stop_requested") then b.stop_requested = omsi.bus.stop_requested() end
  return b
end

-- a place of the game with its tile, so that Omsi-Hub can put it on its own reading of the map
local function reference(b)
  if not (b and b.x and b.y and fn("map.tile_at")) then return nil end
  local tx, ty, lx, ly = omsi.map.tile_at(b.x, b.y)
  if not tx then return nil end
  return { x = b.x, y = b.y, tile_x = tx, tile_y = ty, local_x = lx, local_y = ly }
end

local function map_path()
  if fn("map.info") then
    local m = omsi.map.info()
    if type(m) == "table" and type(m.path) == "string" then return m.path end
  end
  return ""
end

local function date()
  if not fn("world.date") then return "" end
  local d = omsi.world.date()
  if type(d) ~= "table" or not d.year then return "" end
  return string.format("%04d-%02d-%02d", d.year, d.month or 1, d.day or 1)
end

local function clock()
  if fn("world.time") then return num(omsi.world.time()) end
  local i = fn("info") and omsi.info() or nil
  return type(i) == "table" and num(i.clock) or nil
end

local function send_live()
  local b = bus()
  local d, stops = duty()
  local signon = hook.signon
  local live = {
    t = "live", issued = hook.issued and hook.issued() or 0, clock = clock(),
    paused = fn("world.paused") and omsi.world.paused() or false,
    map = map_path(), date = date(), bus = b, ref = reference(b), duty = d,
    signon = signon and signon.stage or "work", signon_wrong = signon and signon.wrong and signon.wrong() or false,
  }
  -- the street at the place Omsi-Hub asked for (the road after the next turn)
  if probe and fn("map.lane") then
    local lane = omsi.map.lane(probe[1], probe[2])
    live.probe = { x = probe[1], y = probe[2], name = type(lane) == "table" and lane.name or "" }
  end
  send(live)
  return stops
end

local function send_dev()
  if fn("has_vehicle") and not omsi.has_vehicle() then return end
  local vars, strs = {}, {}
  if fn("bus.get_vars") then
    for k, v in pairs(omsi.bus.get_vars(IBIS_VARS) or {}) do vars[k] = num(v) end
  elseif fn("var") then
    for _, n in ipairs(IBIS_VARS) do vars[n] = num(omsi.var(n)) end
  end
  -- (the IBIS's strings: its display's lines, what is typed, where it goes)
  local names = fn("vars") and omsi.vars("str") or {}
  local n = 0
  for _, name in ipairs(names) do
    if n < 16 and type(name) == "string" and matches(name, IBIS) then
      local s = fn("str") and omsi.str(name) or nil
      if type(s) == "string" then strs[name], n = s:sub(1, 80), n + 1 end
    end
  end
  if not strs.IBIS_terminus_name and fn("str") then strs.IBIS_terminus_name = omsi.str("IBIS_terminus_name") end
  local dev = { t = "dev", ibis = vars, strs = strs }
  if fn("bus.ticket_request") then
    local name, price = omsi.bus.ticket_request()
    if name then dev.request = { name = name, price = num(price) or 0 } end
  end
  if fn("bus.sales") then
    local count, money = omsi.bus.sales()
    dev.sales = { count = num(count) or 0, money = num(money) or 0 }
  end
  send(dev)
end

-- --- the commands -------------------------------------------------------------------------------

local function press(key)
  if not allowed[key] then return false, "unknown_key" end
  if fn("trigger") then
    omsi.trigger(key)
    return true
  end
  return false, "not_supported"
end

local function carry_out(c)
  local what = c["do"]
  if what == "key" then
    return press(tostring(c.key or ""))
  elseif what == "ticket" then
    local i = math.tointeger(tonumber(c.i)) or 0
    local key = "ticketprinter_button_ticket_" .. i
    if allowed[key] then return press(key) end
    if fn("bus.action") and omsi.bus.action("ticket_give") then return true end
    return false, "no_printer"
  elseif what == "ibis_line" then
    if not fn("bus.set_line") then return false, "not_supported" end
    return omsi.bus.set_line(tostring(c.line or "")) == true or false, "refused"
  elseif what == "type_ibis" then
    if not fn("duty.type_ibis") then return false, "not_supported" end
    omsi.duty.type_ibis()
    return true
  elseif what == "take_duty" then
    if not fn("duty.start") then return false, "not_supported" end
    local ok, why = omsi.duty.start(tostring(c.line or ""), tostring(c.tour or ""), math.tointeger(tonumber(c.trip)) or 1)
    if not ok then return false, tostring(why or "refused") end
    if fn("duty.type_ibis") then omsi.duty.type_ibis() end
    return true
  elseif what == "drive_free" then
    if hook.drive_free then hook.drive_free() end
    return true
  elseif what == "sign_on" then
    local s = hook.signon
    if not s then return false, "no_sign_on" end
    if s.try(c.number, c.code) then
      if hook.changed then hook.changed() end
      return true
    end
    if hook.changed then hook.changed() end
    return false, "wrong"
  elseif what == "accept_order" then
    if not (hook.accept and hook.accept()) then return false, "not_now" end
    return true
  elseif what == "msg" then
    local m = hook.messages
    if not (m and m.answer) then return false, "no_messages" end
    if m.answer(c.kind, c.id, c.key) then return true end
    return false, "no_message"
  end
  return false, "unknown_command"
end

-- the commands Omsi-Hub wrote, each new one done and acked
local function look()
  if not (fn("files.read") and fn("json.decode")) then return end
  local text = omsi.files.read(M.FILE)
  on = text ~= nil and text ~= ""
  if not on then return end
  local ok, q = pcall(omsi.json.decode, text)
  if not ok or type(q) ~= "table" then return end
  port = math.tointeger(tonumber(q.port)) or M.PORT
  probe = type(q.probe) == "table" and num(q.probe[1]) and num(q.probe[2]) and { num(q.probe[1]), num(q.probe[2]) } or nil
  -- (a session seen the first time, when the plugin starts: what is in the file is old)
  if sid == nil then
    sid, done = q.sid, math.tointeger(tonumber(q.seq)) or 0
    return
  end
  if q.sid ~= sid then sid, done = q.sid, 0 end
  for _, c in ipairs(type(q.cmds) == "table" and q.cmds or {}) do
    local seq = math.tointeger(tonumber(c.seq)) or 0
    if seq > done then
      done = seq
      local fine, result, why = pcall(carry_out, c)
      if not fine then result, why = false, "error" end
      send({ t = "ack", seq = seq, ok = result == true, error = result == true and "" or (why or "refused") })
    end
  end
end

-- --- the clock ------------------------------------------------------------------------------------

-- a stop left: the delay there kept for the trip's list
function M.stop_depart(number, delay)
  local n = tonumber(number)
  if n then left_at[n] = num(delay) end
end

-- the bus changed: its keys found again
function M.vehicle()
  keys_for = nil
end

-- every quarter of a second (main.lua)
function M.tick(dt)
  for k, v in pairs(wait) do wait[k] = v - (dt or 0.25) end
  if wait.look <= 0 then
    wait.look = 0.25
    look()
  end
  if not on then return end
  if wait.live <= 0 then
    wait.live = 0.5
    local stops = send_live()
    send_dev()
    -- (the trip's stops whenever they changed: they carry which ones were passed)
    local text = json.encode(stops)
    if not lists.trip or lists.trip.text ~= text or wait.lists <= 0 then send_list("trip", "stops", stops) end
  end
  if wait.keys <= 0 then
    wait.keys = 5
    send_keys()
  end
  -- (the lists again now and then, for an Omsi-Hub started after the game)
  if wait.lists <= 0 then
    wait.lists = 10
    local trips = {}
    for i, t in ipairs(fn("duty.trips") and omsi.duty.trips() or {}) do
      trips[#trips + 1] = { n = i, name = t.name or "", line = t.line or "", terminus = t.terminus or "", departure = num(t.departure), arrival = num(t.arrival) }
    end
    send_list("trips", "trips", trips)
    local lines = {}
    for _, l in ipairs(fn("timetable.lines") and omsi.timetable.lines() or {}) do
      if l.user_allowed ~= false then
        local tours = {}
        for _, t in ipairs(l.tours or {}) do tours[#tours + 1] = { number = tostring(t.number or ""), today = t.today ~= false } end
        lines[#lines + 1] = { name = tostring(l.name or ""), tours = tours }
      end
    end
    send_list("lines", "lines", lines)
  end
end

return M
