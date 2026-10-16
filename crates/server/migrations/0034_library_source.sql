-- Where a stock file came from (#1237). A stock import records it on the
-- asset, in project.json's `source` block (scorsese_core::Source: library,
-- id, author, page_url, licence); the library keeps a copy on the item's row
-- so the details panel can show it without reading a project.
--
-- The document scorsese_core::Source serialises to. NULL means a file the
-- user brought, or anything kept before this: a `pixabay-<id>` name is not
-- read back into one, because the author and page were never stored.
-- The column rides on the user's own row, under 0005's row-level security.
ALTER TABLE library_items
    ADD COLUMN source JSONB CHECK (jsonb_typeof(source) = 'object');
