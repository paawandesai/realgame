-- Anonymous gameplay events. See docs/ANALYTICS.md for what each column
-- means, why it is collected, and when it is deleted.
--
-- Deliberately absent: IP address, user agent, country, timestamps finer than
-- a UTC day, and anything the player typed. Do not add columns without a
-- written purpose and a deletion date (docs/ENGINEERING.md).
CREATE TABLE events (
  id INTEGER PRIMARY KEY,
  -- UTC calendar day the server received the event, "YYYY-MM-DD".
  day TEXT NOT NULL CHECK (length(day) = 10),
  -- 32 random hex characters made on the device. It changes every UTC day, so
  -- it can count a day's unique players but cannot link one day to the next.
  daily_id TEXT NOT NULL CHECK (length(daily_id) = 32),
  type TEXT NOT NULL CHECK (type IN ('session_start', 'room_complete', 'trial_complete', 'session_end')),
  band TEXT NOT NULL CHECK (band IN ('sprout', 'seedling', 'apprentice')),
  -- room_complete only: 0-based room index.
  room INTEGER CHECK (room IS NULL OR (room >= 0 AND room <= 15)),
  -- session_end only: session length bucket, 0 = under 1 min ... 4 = 30 min or more.
  dur INTEGER CHECK (dur IS NULL OR (dur >= 0 AND dur <= 4))
) STRICT;

-- Serves the per-device daily cap, the 30-day stats window and the 90-day purge.
CREATE INDEX events_day_id ON events (day, daily_id);
