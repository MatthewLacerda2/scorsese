-- The library takes text documents (#1510): a `.txt` or `.md` the person
-- wrote elsewhere (a script, a shot list, a briefing), for the assistant to
-- read with `library_read`. A document is kept as kind `text` and is never an
-- asset: `import` refuses one, so no project_assets row ever names it.
-- Widening the CHECK is all the table needs: no row changes.
ALTER TABLE library_items DROP CONSTRAINT library_items_kind_check;
ALTER TABLE library_items
    ADD CONSTRAINT library_items_kind_check
    CHECK (kind IN ('video', 'image', 'audio', 'html', 'text'));

ALTER TABLE uploads DROP CONSTRAINT uploads_kind_check;
ALTER TABLE uploads
    ADD CONSTRAINT uploads_kind_check CHECK (kind IN ('video', 'image', 'audio', 'html', 'text'));
