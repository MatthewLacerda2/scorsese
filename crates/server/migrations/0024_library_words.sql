-- When each word of a spoken line is said (#886, the web half of #811). A
-- line generated locally keeps its timings in a file beside its audio
-- (scorsese_core::words); a library item is one file, so here they are kept
-- on the item's row instead, and written out beside the audio whenever a
-- project is laid out as a folder (projects::media::materialise).
--
-- The document scorsese_core::words::Words serialises to: {"words": [...]}.
-- NULL means no timings, said and never guessed: every item kept before this,
-- every upload, every shot and still, and a line whose provider sent none.
-- Nothing is backfilled -- getting them would mean paying for the line again.
-- The column rides on the user's own row, under 0005's row-level security.
ALTER TABLE library_items
    ADD COLUMN words JSONB CHECK (jsonb_typeof(words) = 'object');
