-- A MIDI file is a library item (#678): uploaded the way a song is, and
-- written there by synth_export. It is not media -- no clip shows it, and it
-- has no thumbnail -- so it is the one kind that never becomes a project
-- asset; synth_import reads it into a recipe instead. `media` stays the empty
-- object, since there is nothing for a prober to find. The argument is in
-- crates/server/src/library/kind.rs.
ALTER TABLE library_items DROP CONSTRAINT library_items_kind_check;
ALTER TABLE library_items
    ADD CONSTRAINT library_items_kind_check CHECK (kind IN ('video', 'image', 'audio', 'midi'));

ALTER TABLE uploads DROP CONSTRAINT uploads_kind_check;
ALTER TABLE uploads
    ADD CONSTRAINT uploads_kind_check CHECK (kind IN ('video', 'image', 'audio', 'midi'));
