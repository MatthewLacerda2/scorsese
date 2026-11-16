-- A visitor is counted once the page saw a person use it (#1797): the landing
-- sends `input` on the first trusted pointer, touch, wheel or key event, and
-- the report (crates/server/src/ab/report.rs) counts only visitors with one.
ALTER TABLE ab_events DROP CONSTRAINT ab_events_kind_check;
ALTER TABLE ab_events ADD CONSTRAINT ab_events_kind_check
    CHECK (kind IN ('view', 'input', 'section', 'end', 'time', 'prompt'));

-- Visitors stored before `input` existed cannot have one, so this migration
-- judges them once, by what the live data showed (#1797): every automated load
-- came from datacenter addresses Cloudflare placed in Virginia and Iowa, and
-- none of them reached a section. Such a visitor is left out; any other is
-- counted. NULL for every visitor from here on, which only `input` counts.
ALTER TABLE ab_visitors ADD COLUMN person_before_input BOOLEAN;
UPDATE ab_visitors v SET person_before_input =
    coalesce(v.country, '') <> 'US'
    OR coalesce(v.region, '') NOT IN ('VA', 'IA')
    OR EXISTS (SELECT 1 FROM ab_events e WHERE e.visitor = v.code AND e.kind = 'section');
