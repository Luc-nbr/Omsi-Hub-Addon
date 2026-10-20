-- Omsi-Hub's plugin: the live dashboard (Luc, 2026-10-19) - what Omsi-Hub's window shows the
-- driver while the game runs, on every drive (a shift, a tour, a free drive, a company's duty):
-- where the bus is and how fast it goes, the duty and its trip with its stops and their times,
-- how late it runs, the passengers on board, and what the drive came to so far (the distance,
-- the stops served on time, the passengers who got on).
--
-- Nothing is drawn in the game: one file for Omsi-Hub, written every few seconds where the game
-- has `files.write` (`data/dash.json`, hub-game's `bridge::Dash`; hub-api's `live.rs` reads it).

local json = require("hubjson")
local career = require("career")

local M = {}

M.FILE = "dash.json"
local EVERY = 2

local trip, assignment, has = nil, nil, {}
local wait = 0

local function fn(t, name)
  return type(t) == "table" and type(t[name]) == "function"
end

function M.setup(trip_module, a, functions)
  trip, assignment, has = trip_module, a, functions or {}
end

local function num(v)
  v = tonumber(v)
  if v == nil or v ~= v or v == math.huge or v == -math.huge then return nil end
  return v
end

-- what the bus says of itself (each where the game has it)
local function bus()
  local b = { speed = num(omsi.speed and omsi.speed()) or 0 }
  if fn(omsi, "position") then
    local x, y, _, heading = omsi.position()
    if num(x) and num(y) then b.x, b.y, b.heading = num(x), num(y), num(heading) end
  end
  if fn(omsi.bus, "file") then b.file = omsi.bus.file() or "" end
  if fn(omsi.bus, "passengers") then b.passengers = num(omsi.bus.passengers()) end
  if fn(omsi.bus, "odometer") then b.odometer = num(omsi.bus.odometer()) end
  if fn(omsi.bus, "fuel") then b.fuel = num(omsi.bus.fuel()) end
  return b
end

-- what the bus's IBIS is set to (its scripts' variables; nil on a bus without them)
local function ibis()
  if not fn(omsi, "var") then return nil end
  local i = { line = num(omsi.var("IBIS_LinieKurs")), route_index = num(omsi.var("IBIS_RouteIndex")), terminus = num(omsi.var("IBIS_TerminusCode")) }
  if fn(omsi, "str") then
    local n = omsi.str("IBIS_terminus_name")
    if type(n) == "string" and n ~= "" then i.terminus_name = n end
  end
  if i.line == nil and i.route_index == nil and i.terminus == nil then return nil end
  return i
end

-- the duty under way and its trip's stations (none on a free drive)
local function duty()
  local d = fn(omsi.duty, "get") and omsi.duty.get() or nil
  if type(d) ~= "table" then return nil, nil end
  local out = {
    line = d.line or "", tour = d.tour or "", trip = math.tointeger(d.trip) or 0, trips = math.tointeger(d.trips) or 0,
    trip_name = d.trip_name or "", delay = num(d.delay) or 0, at_stop = d.at_stop == true,
  }
  local stops = {}
  for _, s in ipairs(fn(omsi.duty, "stops") and omsi.duty.stops() or {}) do
    stops[#stops + 1] = { name = s.name or "", arrival = num(s.arrival or s.arr) or 0, departure = num(s.departure or s.dep) or 0, stops = s.stops ~= false }
  end
  out.next = d.next_stop and math.max((tonumber(d.next_stop.number) or 1) - 1, 0) or #stops
  if fn(omsi.duty, "trips") then
    local t = omsi.duty.trips()[d.trip]
    if type(t) == "table" then
      out.number, out.terminus = t.line or "", t.terminus or ""
      out.departure, out.arrival = num(t.departure), num(t.arrival)
      if out.trip_name == "" then out.trip_name = t.name or "" end
    end
  end
  return out, stops
end

-- the file for Omsi-Hub (`bridge::Dash`)
function M.view()
  local d, stops = duty()
  local t = trip and trip.totals() or {}
  local counts = career.tally(trip and trip.served_on_drive() or {})
  local i = fn(omsi, "info") and omsi.info() or nil
  return {
    issued = assignment and tonumber(assignment.issued) or 0,
    at = fn(omsi.util, "now") and omsi.util.now() or 0,
    clock = type(i) == "table" and num(i.clock) or nil,
    bus = bus(),
    ibis = ibis(),
    duty = d,
    -- (an empty list would be written as `{}`, which is no list: left out then)
    stops = stops and #stops > 0 and stops or nil,
    drive = {
      seconds = t.seconds or 0, metres = t.metres or 0, jolts = t.jolts or 0, crashes = t.crashes or 0,
      boarded = t.boarded or 0, served = counts.early + counts.on_time + counts.late,
      early = counts.early, on_time = counts.on_time, late = counts.late,
      since_stop = trip and trip.metres_since_stop() or 0,
    },
  }
end

-- every second (main.lua), with the player in a bus
function M.tick(dt)
  if not has["files.write"] then return end
  wait = wait - (dt or 1)
  if wait > 0 then return end
  wait = EVERY
  omsi.files.write(M.FILE, json.encode(M.view()))
end

return M
