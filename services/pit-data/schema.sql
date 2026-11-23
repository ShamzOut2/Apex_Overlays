-- Shared pit stop times. One row per stop: the car and the stop's numbers, nothing else.
-- No driver, account, key, install id, IP address, session or track is stored.
CREATE TABLE IF NOT EXISTS stops (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  game TEXT NOT NULL,          -- 'iracing'
  car_id INTEGER NOT NULL,     -- the sim's car id
  tyres INTEGER NOT NULL,      -- tyres changed, 0-4
  fuel_l REAL NOT NULL,        -- litres put in
  fuel_secs REAL NOT NULL,     -- seconds the fuel took
  fuel_delay REAL NOT NULL,    -- seconds from stopping to the fuel starting
  stood REAL NOT NULL,         -- seconds stood in the box
  day TEXT NOT NULL            -- the date received (YYYY-MM-DD), to drop old rows
);
CREATE INDEX IF NOT EXISTS stops_car ON stops (game, car_id, id);
