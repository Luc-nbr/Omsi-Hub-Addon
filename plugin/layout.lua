-- Omsi-Hub's plugin: the size and the place of its panels, the driver's own. While the panels
-- have the mouse (F10) each of them shows a row under its content: smaller, its size, larger,
-- and back to how it came. The size multiplies the panel's sizes (its width, padding and gaps,
-- its texts, icons, bars and charts; openOMSI's badges and buttons keep theirs). A panel the
-- driver dragged keeps its place in later drives: where it was dragged to is kept with its
-- size in `omsi.data.layout`, and given as the panel's offset when it comes on the screen again.
--
-- The board in the navigator (`anchor = "navigator"`) is the navigator's: it goes as it is, at
-- the navigator's size and place.

local M = {}

local MIN, MAX, STEP = 0.6, 2, 0.1
-- (the defaults of openOMSI's panel table, and of its elements, that a size multiplies)
local PANEL_SIZES = { width = 300, padding = 12, gap = 6, radius = 12 }
local SIZES = {
  text = { size = 14 }, icon = { size = 20 }, row = { gap = 8 }, box = { gap = 4 },
  space = { size = 8 }, bar = { height = 6 }, chart = { height = 48 }, image = { width = 64, height = 64 },
  stops = { size = 13 }, table = { size = 13 },
}
-- (an anchor's place across and down: 0 the left or top, 1 the right or bottom)
local FACTORS = {
  top_left = { 0, 0 }, top = { 0.5, 0 }, top_right = { 1, 0 },
  left = { 0, 0.5 }, center = { 0.5, 0.5 }, right = { 1, 0.5 },
  bottom_left = { 0, 1 }, bottom = { 0.5, 1 }, bottom_right = { 1, 1 },
}
local DIM = "#959DB0"

local tr = function(t) return t end
-- (by panel: the table it was last given, and the offset it came on the screen with)
local last, base = {}, {}

function M.setup(translate)
  tr = translate
end

local function kept(id)
  omsi.data.layout = type(omsi.data.layout) == "table" and omsi.data.layout or {}
  local k = omsi.data.layout[id]
  if type(k) ~= "table" then
    k = { scale = 1, dx = 0, dy = 0 }
    omsi.data.layout[id] = k
  end
  k.scale = math.min(math.max(tonumber(k.scale) or 1, MIN), MAX)
  k.dx, k.dy = tonumber(k.dx) or 0, tonumber(k.dy) or 0
  return k
end

-- how far the player dragged the panel `id` on the screen now (nil: it is not there)
local function dragged(id)
  if not (omsi.ui and omsi.ui.moved) then return 0, 0 end
  local dx, dy = omsi.ui.moved(id)
  if dx == nil then return nil end
  return tonumber(dx) or 0, tonumber(dy) or 0
end

local function focused()
  return omsi.ui and omsi.ui.focused and omsi.ui.focused() or false
end

-- an element (and the ones in it) at the size `k`
local function sized(e, k)
  if type(e) ~= "table" then return e end
  local out = {}
  for key, v in pairs(e) do out[key] = v end
  for key, default in pairs(SIZES[e.type] or {}) do out[key] = (tonumber(e[key]) or default) * k end
  if type(e.children) == "table" then
    local children = {}
    for i, c in ipairs(e.children) do children[i] = sized(c, k) end
    out.children = children
  end
  return out
end

-- the panel's table at the size `k`, moved by (dx, dy) on the screen
function M.sized(t, k, dx, dy)
  local out = {}
  for key, v in pairs(t) do out[key] = v end
  for key, default in pairs(PANEL_SIZES) do out[key] = (tonumber(t[key]) or default) * k end
  local children = {}
  for i, c in ipairs(t.children or {}) do children[i] = sized(c, k) end
  out.children = children
  local f = FACTORS[t.anchor or "top_left"] or FACTORS.top_left
  out.x = (tonumber(t.x) or 16) + (f[1] == 1 and -dx or dx)
  out.y = (tonumber(t.y) or 16) + (f[2] == 1 and -dy or dy)
  return out
end

local function controls(id, k)
  return {
    { type = "divider" },
    { type = "row", gap = 6, children = {
      { type = "icon", name = "open_with", size = 16, color = DIM },
      { type = "text", text = tr("Size"), size = 12, weight = "bold", color = DIM, grow = true, wrap = false },
      { type = "button", id = "hub_size_down", icon = "remove", visible = k > MIN + 1e-6 },
      { type = "text", id = "hub_size", text = string.format("%d %%", math.floor(k * 100 + 0.5)), size = 12, weight = "bold", align = "center" },
      { type = "button", id = "hub_size_up", icon = "add", visible = k < MAX - 1e-6 },
      { type = "button", id = "hub_size_reset", icon = "restart_alt" },
    } },
  }
end

-- Puts the panel `id` on the screen at the driver's size and place (`t`: its table, as
-- `omsi.ui.set` takes it).
function M.set(id, t)
  last[id] = t
  if t.anchor == "navigator" then return omsi.ui.set(id, t) end
  local k = kept(id)
  local dx, dy = dragged(id)
  if dx == nil or base[id] == nil then
    -- (on the screen again: where the driver left it last)
    base[id] = { k.dx, k.dy }
    dx, dy = 0, 0
  end
  k.dx, k.dy = base[id][1] + dx, base[id][2] + dy
  local shown = M.sized(t, k.scale, base[id][1], base[id][2])
  if focused() and t.draggable then
    local children = {}
    for i, c in ipairs(shown.children) do children[i] = c end
    for _, c in ipairs(controls(id, k.scale)) do children[#children + 1] = c end
    shown.children = children
  end
  return omsi.ui.set(id, shown)
end

-- Takes the panel `id` off the screen; where it was dragged to is kept for the next time.
function M.remove(id)
  if last[id] and base[id] then
    local dx, dy = dragged(id)
    if dx then
      local k = kept(id)
      k.dx, k.dy = base[id][1] + dx, base[id][2] + dy
    end
  end
  last[id], base[id] = nil, nil
  if omsi.ui and omsi.ui.remove then return omsi.ui.remove(id) end
  return false
end

-- Sets every panel again (the panels got or gave back the mouse: the size row comes or goes).
local was_focused = false
function M.refresh()
  was_focused = focused()
  for id, t in pairs(last) do M.set(id, t) end
end

-- The panels got or gave back the mouse some other way (Esc, a menu of the game): set again.
function M.watch()
  if focused() ~= was_focused then M.refresh() end
end

-- Keeps where every panel was dragged to (the game ends, the plugin stops).
function M.keep()
  for id in pairs(last) do
    local dx, dy = dragged(id)
    if dx and base[id] then
      local k = kept(id)
      k.dx, k.dy = base[id][1] + dx, base[id][2] + dy
    end
  end
end

-- A click on a panel's size row: handled (true) or not the layout's.
function M.click(id, element)
  if type(element) ~= "string" or not element:match("^hub_size") or not last[id] then return false end
  local k = kept(id)
  if element == "hub_size_down" then
    k.scale = math.max(k.scale - STEP, MIN)
  elseif element == "hub_size_up" then
    k.scale = math.min(k.scale + STEP, MAX)
  elseif element == "hub_size_reset" then
    -- (back where its table puts it: the drag of this session undone with the offset)
    local dx, dy = dragged(id)
    k.scale = 1
    base[id] = { -(dx or 0), -(dy or 0) }
  end
  k.scale = math.floor(k.scale * 10 + 0.5) / 10
  M.set(id, last[id])
  return true
end

return M
