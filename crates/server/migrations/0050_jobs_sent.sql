-- When a paid generation's call went out to its vendor, committed before the
-- call (#1645). A job that comes back from a stop with it set and nothing kept
-- is never sent again: it fails, and its reservation is released.
ALTER TABLE jobs ADD COLUMN sent_at TIMESTAMPTZ;
