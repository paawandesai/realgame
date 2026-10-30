-- Adds the quest_complete event type (the first-person 3D client, I-005).
-- See docs/ANALYTICS.md.
--
-- SQLite cannot change a CHECK constraint in place, so the table is rebuilt:
-- same columns, same order, same CHECKs, still STRICT, with 'quest_complete'
-- added to the type list. No column is added and no new kind of data is
-- stored: a quest index goes in the existing `room` column. Never edit
-- 0001_events.sql, which is already deployed.
--
-- Deliberately absent, as before: IP address, user agent, country,
-- timestamps finer than a UTC day, and anything the player typed. Do not add
-- columns without a written purpose and a deletion date (docs/ENGINEERING.md).
CREATE TABLE events_new (
  id INTEGER PRIMARY KEY,
  -- UTC calendar day the server received the event, "YYYY-MM-DD".
  day TEXT NOT NULL CHECK (length(day) = 10),
  -- 32 random hex characters made on the device. It changes every UTC day, so
  -- it can count a day's unique players but cannot link one day to the next.
  daily_id TEXT NOT NULL CHECK (length(daily_id) = 32),
  -- Event type: one of a fixed list.
  type TEXT NOT NULL CHECK (type IN ('session_start', 'quest_complete', 'room_complete', 'trial_complete', 'session_end')),
  -- The age band the player picked: one of three fixed values, never an age.
  band TEXT NOT NULL CHECK (band IN ('sprout', 'seedling', 'apprentice')),
  -- Room or quest index, 0 to 15, set only for room_complete and quest_complete.
  room INTEGER CHECK (room IS NULL OR (room >= 0 AND room <= 15)),
  -- session_end only: session length bucket, 0 = under 1 min ... 4 = 30 min or more.
  dur INTEGER CHECK (dur IS NULL OR (dur >= 0 AND dur <= 4))
) STRICT;

INSERT INTO events_new (id, day, daily_id, type, band, room, dur)
  SELECT id, day, daily_id, type, band, room, dur FROM events;

DROP TABLE events;
ALTER TABLE events_new RENAME TO events;

-- Serves the per-device daily cap, the 30-day stats window and the 90-day purge.
CREATE INDEX events_day_id ON events (day, daily_id);
