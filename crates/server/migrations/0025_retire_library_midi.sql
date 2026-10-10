-- The web keeps no MIDI files any more (#964). Since #785 took synth_import
-- and synth_export off the tool list, nothing on the web reads or writes
-- one: a `.mid` was accepted at upload and then of no use. Uploads of one
-- are refused (415) like any other unsupported type, and the kind 0017 added
-- goes.
--
-- Any row stored as `midi` is deleted first, so no row can hold a kind the
-- server no longer reads -- the reason #961 kept the variant. On the hosting
-- machine there were none (counted 2026-10-10). A MIDI item was never
-- imported, so no project asset names one and nothing refers to it; its file
-- under users/<id>/library/ is not something SQL can reach, and with no rows
-- there is no file either. An upload still in flight as `midi` goes too: its
-- bytes are in the rebuildable cache.
DELETE FROM library_items WHERE kind = 'midi';
DELETE FROM uploads WHERE kind = 'midi';

ALTER TABLE library_items DROP CONSTRAINT library_items_kind_check;
ALTER TABLE library_items
    ADD CONSTRAINT library_items_kind_check CHECK (kind IN ('video', 'image', 'audio'));

ALTER TABLE uploads DROP CONSTRAINT uploads_kind_check;
ALTER TABLE uploads
    ADD CONSTRAINT uploads_kind_check CHECK (kind IN ('video', 'image', 'audio'));
