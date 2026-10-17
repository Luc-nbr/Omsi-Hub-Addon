-- A duty of several parts (Omsi-Hub's `legs`, src/bridge.rs `Leg`): a shift put together of
-- tours, or a company's duty that begins or ends with a relief on the way. openOMSI's command
-- line starts the first part's tour; this takes each next part with `duty.start` when the part
-- before it is done, and ends the duty after the last one - what the old game's `--duty-leg`
-- did inside the game.
--
-- A part is { line, tour, first, trips, from_stop, to_stop }: trips `first` to
-- `first + trips - 1` of the tour (from 1), the first taken from its stop `from_stop` on and the
-- last driven to its stop `to_stop` (from 1; 0: the trip's own first and last stop).

local M = {}

local legs = {}
-- the part under way (0: not begun), and whether its first trip was put at its relief stop
local at = 0
local placed = false

local function num(v, default)
  local n = tonumber(v)
  if n == nil then return default end
  return math.floor(n)
end

function M.setup(assignment)
  legs, at, placed = {}, 0, false
  if type(assignment) ~= "table" or type(assignment.legs) ~= "table" then return end
  for _, l in ipairs(assignment.legs) do
    if type(l) == "table" and (l.line or "") ~= "" then
      legs[#legs + 1] = {
        line = l.line, tour = tostring(l.tour or ""), first = math.max(1, num(l.first, 1)),
        trips = math.max(1, num(l.trips, 1)), from_stop = num(l.from_stop, 0), to_stop = num(l.to_stop, 0),
      }
    end
  end
end

function M.any()
  return #legs > 0
end

function M.count()
  return #legs
end

-- the part under way (from 1) and the part itself
function M.current()
  return at, legs[at]
end

local function last_trip(l)
  return l.first + l.trips - 1
end

local function same(a, b)
  return tostring(a or ""):lower() == tostring(b or ""):lower()
end

-- the duty the game has now (`line`, `tour`; nil: none): the part it is, begun. Returns the
-- stop the part's first trip is to be taken from when the game started it at the trip's first
-- stop (a relief on the way), once.
function M.duty_began(line, tour)
  if #legs == 0 or not line then return nil end
  if at == 0 then
    for k, l in ipairs(legs) do
      if same(l.line, line) and same(l.tour, tour) then
        at = k
        break
      end
    end
    if at == 0 then return nil end
  end
  local l = legs[at]
  if l and not placed and same(l.line, line) and same(l.tour, tour) then
    placed = true
    if l.from_stop > 1 then return l.from_stop end
  end
  return nil
end

-- the first part, to start it (the game did not: an older openOMSI, or no tour on its command
-- line): line, tour, trip, stop
function M.first()
  local l = legs[1]
  if not l then return nil end
  at, placed = 1, true
  return l.line, l.tour, l.first, l.from_stop
end

-- trip `n` of the part under way is done: what comes next - "next" with the part to start,
-- "done" when the duty ends with it, nil when the part goes on
function M.trip_done(n)
  local l = legs[at]
  if not l or (tonumber(n) or 0) < last_trip(l) then return nil end
  -- (a part that ends at a relief stop ends there, not at the trip's end: `relief_at`)
  if l.to_stop > 0 then return nil end
  return M.advance()
end

-- the part under way ended: the next one ("next", its line, tour, first trip and stop), or "done"
function M.advance()
  at = at + 1
  placed = false
  local l = legs[at]
  if not l then return "done" end
  placed = true
  return "next", l.line, l.tour, l.first, l.from_stop
end

-- the bus at stop `number` (from 1) of trip `trip`: whether the part under way ends here, at its
-- relief
function M.relief_at(trip, number)
  local l = legs[at]
  if not l or l.to_stop <= 0 then return false end
  return (tonumber(trip) or 0) == last_trip(l) and (tonumber(number) or 0) >= l.to_stop
end

return M
