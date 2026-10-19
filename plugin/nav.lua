-- Omsi-Hub's plugin: the navigator as one unit (Luc, 2026-10-19) - openOMSI's navigator with
-- Omsi-Hub's part under its map, laid out as a phone: a status line on top (the line's plate,
-- where the trip goes, how it stands against the timetable, the clock) and five tabs at the
-- bottom -
--   Navigate  the stop behind the bus and the next ones, what comes after this trip; at a
--             terminus with time to spare, the break clock counting down to the next departure
--   Stops     every stop of the trip still to come, with its time
--   Trips     the duty's trips, the one under way marked, and the duty strip's next break and relief
--   Duty      signing on (where Omsi-Hub asks for it), the duty order, starting and signing off,
--             the notes and the walk-around of the duty strip
--   Messages  the control room's messages, answered here (a new one makes the tab blink with
--             its count; an instruction waiting for an answer opens it)
-- The board, the sign-on, the duty strip and the inbox are their own modules: this one puts them
-- together in the navigator and hands their clicks back to them.
--
-- It fits the navigator it is in, which the player sizes and turns: the room under the map sets
-- how many stops a tab shows; a narrow navigator gets the tabs as icons only, a short one only
-- the status line and the one thing that matters now (the next stop, the break, a message) - the
-- slim strip. An openOMSI without the navigator's board (`omsi.ui.version` below 3), a navigator
-- switched off or its board taken by another plugin: the modules' panels of their own, as before.

local board = require("board")
local company = require("company")
local layout = require("layout")
local messages = require("messages")
local signon = require("signon")
local strip = require("duty_strip")

local M = {}

local PANEL = "omsihub_nav"
M.PANEL = PANEL

local TABS = { "navigate", "stops", "trips", "duty", "messages" }
local LABELS = { navigate = "Navigate", stops = "Stops", trips = "Trips", duty = "Duty", messages = "Messages" }
local ICONS = { navigate = "navigation", stops = "departure_board", trips = "route", duty = "badge", messages = "forum" }
-- the navigator's look in Omsi-Hub's colours (the mockup): its blue, the line's yellow plate
local ACCENT, ON_ACCENT, TAB = "#2A75F7", "#FFFFFF", "#FFFFFF14"
local TEXT, DIM, FAINT = "#E8EBF2", "#959DB0", "#6E7688"
local OK, WARN, LATE = "#37D67A", "#F0B429", "#FF5F57"
-- (narrower than this the tabs are icons; shorter than this the slim strip)
local WIDE = 330
local SLIM = 150
-- (how long the board may stay away before the modules get their own panels back, s)
local GRACE = 3

local tr = function(t) return t end
local hooks = {}
local shown = false
local docked = false
local away = 0
-- (the board's width and room, as last seen)
local size = {}
-- (stops the Navigate and Stops tabs show, fitted to the room)
local fit = { navigate = 4, stops = 8 }
-- (what was seen last: the break clock comes up as a break begins, the inbox as an instruction
-- comes, the Duty tab as the walk-around opens or the driver has to sign on - once each, and
-- the driver may go to another tab)
local seen = { phase = nil, wants = false, form = nil, work = true }

omsi.data.nav_tab = omsi.data.nav_tab or "navigate"

-- `translate` the plugin's texts; `h` what main.lua knows: `duty_children()` (the company's
-- panel's elements), `wanted()` (the unit is wanted: a duty or an inbox), `boarded()` the
-- passengers, `parts()` and `assignment()` for the company's parts
function M.setup(translate, h)
  tr, hooks = translate, h or {}
end

local function hhmm(t)
  local m = math.floor((t or 0) / 60)
  return string.format("%02d:%02d", (m // 60) % 24, m % 60)
end

local function mmss(s)
  s = math.max(math.floor(s or 0), 0)
  return string.format("%d:%02d", s // 60, s % 60)
end

local function clock()
  local i = omsi.info and omsi.info() or nil
  return type(i) == "table" and tonumber(i.clock) or 0
end

-- whether this openOMSI has the navigator's board
local function docks()
  return omsi.ui and omsi.ui.set and (tonumber(omsi.ui.version) or 0) >= 3
end

-- the board under the navigator's map: width, room, height, whether it is this plugin's
local function dock()
  if not (omsi.ui and omsi.ui.dock) then return nil end
  local w, room, h, mine = omsi.ui.dock()
  if not w then return nil end
  return w, room or 0, h or 0, mine ~= false
end

-- whether the unit is in the navigator now (the modules keep their own panels otherwise), and
-- whether it asks for the navigator's board at all (the board module then does not)
function M.active()
  return shown and docked
end

function M.asking()
  return shown
end

-- the tab shown
function M.tab()
  return omsi.data.nav_tab
end

function M.show(tab)
  if LABELS[tab] then omsi.data.nav_tab = tab end
  if tab == "messages" then messages.seen() end
end

-- --- the parts of the unit ----------------------------------------------------------------------

-- the status line: the line's plate, where the trip goes, the chip, the clock
local function status_line(d, trips, now)
  local children
  if d then
    local phase, chip, fill, ink = board.phase(d, trips, now)
    local k = phase == "break" and d.trip + 1 or d.trip
    local trip = trips[k] or {}
    local line = trip.line or d.line or ""
    children = {
      board.plate(line),
      { type = "text", text = board.terminus_of(line, trip.terminus), size = 14, weight = "bold", color = TEXT, grow = true, wrap = false },
      { type = "badge", text = chip, color = fill, text_color = ink, weight = "bold" },
    }
  else
    local a = hooks.assignment and hooks.assignment() or nil
    local name = a and (a.company or "") ~= "" and a.company or "Omsi-Hub"
    children = {
      { type = "icon", name = "directions_bus", size = 18, color = ACCENT },
      { type = "text", text = name, size = 14, weight = "bold", color = TEXT, grow = true, wrap = false },
    }
  end
  children[#children + 1] = { type = "text", text = hhmm(now), size = 13, weight = "bold", color = DIM, wrap = false }
  return { type = "row", gap = 8, children = children }
end

-- the break clock: what is left until the next departure, large, and how much of the break is
-- gone; the trip that comes next
local function break_clock(d, trips, now)
  local this, after = trips[d.trip] or {}, trips[d.trip + 1] or {}
  local left = (after.departure or now) - now
  local length = math.max((after.departure or 0) - (this.arrival or 0), 1)
  local over = left < 0
  local out = {
    { type = "row", gap = 8, children = {
      { type = "icon", name = "schedule", size = 18, color = over and LATE or WARN },
      { type = "text", text = over and tr("Break over") or tr("Break"), size = 13, weight = "bold", color = over and LATE or WARN, grow = true },
      { type = "text", text = tr("until %{time}", { time = hhmm(after.departure) }), size = 12, color = DIM, wrap = false },
    } },
    { type = "text", text = (over and "+" or "") .. mmss(math.abs(left)), size = 40, weight = "bold", align = "center", color = over and LATE or TEXT },
    { type = "bar", value = math.min(math.max(1 - left / length, 0), 1), height = 6, color = over and LATE or ACCENT, background = TAB },
    { type = "row", gap = 7, children = {
      { type = "text", text = tr("Next %{time}", { time = hhmm(after.departure) }), size = 12.5, weight = "bold", color = TEXT },
      board.plate(after.line or ""),
      { type = "text", text = board.terminus_of(after.line or "", after.terminus), size = 12.5, color = TEXT, grow = true, wrap = false },
    } },
  }
  if (after.line or "") ~= (this.line or "") and (after.line or "") ~= "" then
    out[#out + 1] = { type = "box", color = ACCENT, children = {
      { type = "text", text = tr("Continue on line %{line}, tour %{tour}", { line = after.line, tour = d.tour or "" }), size = 12, weight = "bold", color = TEXT },
    } }
  end
  return out
end

local function navigate(d, trips, now, roomy)
  if not d then
    return { { type = "text", text = signon.at_work() and tr("Not started yet") or tr("Not signed on"), size = 13, color = DIM } }
  end
  if board.phase(d, trips, now) == "break" then return break_clock(d, trips, now) end
  local rows = board.rows(d, trips, omsi.duty.stops(), now, fit.navigate, false)
  -- (the status line has the plate, the terminus and the chip: the board's own first row goes)
  table.remove(rows, 1)
  if roomy and hooks.parts then
    for _, x in ipairs(company.elements(hooks.parts(), hooks.assignment(), d, trips, now, hooks.boarded(), fit.navigate > 1)) do rows[#rows + 1] = x end
  end
  return rows
end

local function stops(d, trips, now)
  if not d then return { { type = "text", text = tr("No trip under way"), size = 13, color = DIM } } end
  local rows = board.rows(d, trips, omsi.duty.stops(), now, fit.stops, false, 128)
  table.remove(rows, 1)
  return rows
end

local function trip_list(d, trips)
  local out = {}
  if #trips == 0 then
    out[1] = { type = "text", text = tr("No trips yet: start your duty"), size = 13, color = DIM }
  else
    local rows = {}
    for k, t in ipairs(trips) do
      local done = d and (k < d.trip or (k == d.trip and d.trip_done))
      local now = d and k == d.trip and not d.trip_done
      local tint = done and FAINT or (now and TEXT or DIM)
      rows[#rows + 1] = {
        { text = done and "✓" or (now and "▶" or tostring(k)), color = done and OK or (now and ACCENT or FAINT) },
        { text = t.line or "", color = tint },
        { text = hhmm(t.departure), color = tint },
        { text = board.terminus_of(t.line or "", t.terminus), color = tint },
        { text = hhmm(t.arrival), color = tint },
      }
    end
    out[1] = { type = "table", columns = { "", tr("Line"), tr("Dep."), "", tr("Arr.") }, widths = { 0.08, 0.14, 0.16, 0.46, 0.16 },
      aligns = { "center", "left", "left", "left", "right" }, rows = rows, size = 12.5 }
  end
  -- (the duty strip's look ahead: the next break, the relief, the end)
  local s = strip.children()
  for k = 2, #s do
    local e = s[k]
    if e.type == "row" and e.children and e.children[1] and e.children[1].type == "icon" then out[#out + 1] = e end
  end
  return out
end

local function duty_tab()
  local out = hooks.duty_children and hooks.duty_children() or {}
  if signon.at_work() then
    local s = strip.children()
    if #s > 0 then
      out[#out + 1] = { type = "divider" }
      for _, e in ipairs(s) do out[#out + 1] = e end
    end
  end
  if #out == 0 then out[1] = { type = "text", text = tr("No duty: take one in the Omsi-Hub app"), size = 13, color = DIM } end
  return out
end

local function inbox()
  if not messages.has_inbox() then return { { type = "text", text = tr("No messages"), size = 13, color = DIM } } end
  return messages.children()
end

-- the tab bar: a button a tab, the one shown in the accent; icons only where it is narrow; the
-- messages' count on theirs
local function tab_bar(wide)
  local row = {}
  local unseen = messages.unseen()
  for _, t in ipairs(TABS) do
    local label = wide and tr(LABELS[t]) or ""
    if t == "messages" and unseen > 0 and omsi.data.nav_tab ~= "messages" then label = (wide and (label .. " ") or "") .. unseen end
    row[#row + 1] = { type = "button", id = "nav_" .. t, icon = ICONS[t], text = label, grow = true,
      color = omsi.data.nav_tab == t and ACCENT or (t == "messages" and unseen > 0 and WARN .. "55" or TAB) }
  end
  return { type = "row", gap = 4, children = row }
end

-- the slim strip: the one thing that matters now
local function slim(d, trips, now)
  if messages.wants() or messages.unseen() > 0 then
    return { type = "row", gap = 6, children = {
      { type = "icon", name = "forum", size = 16, color = WARN },
      { type = "text", text = tr("Message from control"), size = 12.5, weight = "bold", color = WARN, grow = true, wrap = false },
      { type = "button", id = "nav_messages", icon = "forum", text = "", color = ACCENT },
    } }
  end
  if d and board.phase(d, trips, now) == "break" then
    local after = trips[d.trip + 1] or {}
    return { type = "row", gap = 6, children = {
      { type = "icon", name = "schedule", size = 16, color = WARN },
      { type = "text", text = tr("Break"), size = 12.5, weight = "bold", color = WARN, grow = true },
      { type = "text", text = mmss((after.departure or now) - now), size = 14, weight = "bold", color = TEXT },
    } }
  end
  if d then
    for _, s in ipairs(omsi.duty.stops() or {}) do
      if s.stops ~= false and not s.passed then
        return { type = "row", gap = 6, children = {
          { type = "icon", name = "near_me", size = 16, color = ACCENT },
          { type = "text", text = s.name or "", size = 12.5, weight = "bold", color = TEXT, grow = true, wrap = false },
          { type = "text", text = hhmm(s.arrival), size = 12.5, color = DIM },
        } }
      end
    end
  end
  return nil
end

-- --- the unit, every second ---------------------------------------------------------------------

-- the elements of the unit now, for a board `w` wide with `room` under the map
function M.children(w, room)
  local d = omsi.duty and omsi.duty.get and omsi.duty.get() or nil
  local trips = d and omsi.duty.trips() or {}
  local now = clock()
  -- (what happened brings its tab up: a break beginning, an instruction to answer, the
  -- walk-around at sign-off, a driver still to sign on)
  local phase = d and board.phase(d, trips, now) or nil
  local wants, form, work = messages.wants(), strip.form(), signon.at_work()
  if phase == "break" and seen.phase ~= "break" then M.show("navigate") end
  if form == "check" and seen.form ~= "check" then M.show("duty") end
  if not work and seen.work then M.show("duty") end
  if wants and not seen.wants then M.show("messages") end
  seen.phase, seen.wants, seen.form, seen.work = phase, wants, form, work
  if omsi.data.nav_tab == "messages" then messages.seen() end

  local children = { status_line(d, trips, now) }
  if room and room < SLIM then
    local one = slim(d, trips, now)
    if one then children[#children + 1] = one end
    return children
  end
  children[#children + 1] = { type = "divider", color = TAB }
  local tab = omsi.data.nav_tab
  local body
  if tab == "stops" then body = stops(d, trips, now)
  elseif tab == "trips" then body = trip_list(d, trips)
  elseif tab == "duty" then body = duty_tab()
  elseif tab == "messages" then body = inbox()
  else body = navigate(d, trips, now, (room or 0) > 300) end
  for _, e in ipairs(body) do children[#children + 1] = e end
  children[#children + 1] = { type = "space", size = 2 }
  children[#children + 1] = tab_bar((w or WIDE) >= WIDE)
  return children
end

-- the unit now (`wanted`: a duty or an inbox to show). Whether it is in the navigator; when not,
-- the modules show their own panels.
function M.update(wanted)
  if not (wanted and docks()) then
    if shown then layout.remove(PANEL) end
    shown, docked, away = false, false, 0
    messages.dock(false)
    strip.dock(false)
    return false
  end
  local w, room, h, mine = dock()
  if w and mine then
    away = 0
    -- (the stops the tabs list: as many as there is room for, found from how tall it came out)
    local tab = omsi.data.nav_tab == "stops" and "stops" or "navigate"
    if h > 0 and room > 0 then
      if h > room and fit[tab] > 1 then
        fit[tab] = fit[tab] - 1
      elseif h + 26 < room and fit[tab] < (tab == "stops" and 64 or 4) then
        fit[tab] = fit[tab] + 1
      end
    end
  else
    -- (no board: the navigator is off, Shift+N put it away or another plugin has it; asked for
    -- once more, for the first time it opens by itself)
    away = away + 1
  end
  docked = away <= GRACE
  size = { w, room }
  layout.set(PANEL, { anchor = "navigator", padding = 10, gap = 6, children = M.children(w, room) })
  messages.dock(docked)
  strip.dock(docked)
  shown = true
  return docked
end

-- the unit drawn again at once (a click on it)
function M.refresh()
  if shown then layout.set(PANEL, { anchor = "navigator", padding = 10, gap = 6, children = M.children(size[1], size[2]) }) end
end

-- a click on the unit: a tab, or what the module whose element it is takes (`messages`, the
-- strip, main.lua's own for the rest). Returns the panel the click belongs to.
function M.route(p, element)
  if p ~= PANEL then return p end
  local e = element or ""
  local tab = e:match("^nav_(%a+)$")
  if tab then
    M.show(tab)
    if tab ~= "messages" then messages.hide() end
    return nil
  end
  if e:sub(1, 4) == "msg_" then return messages.PANEL end
  if e:sub(1, 6) == "strip_" then return strip.PANEL end
  return "omsihub"
end

-- F9: the Messages tab shown, or the tab before it again
local before = "navigate"
function M.key(key, down)
  if not (M.active() and key == "F9" and down) then return false end
  if omsi.data.nav_tab == "messages" then
    M.show(before)
  else
    before = omsi.data.nav_tab
    M.show("messages")
  end
  return true
end

function M.stop()
  if shown then layout.remove(PANEL) end
  shown, docked = false, false
end

return M
