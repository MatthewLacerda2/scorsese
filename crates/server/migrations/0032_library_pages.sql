-- The library takes web pages (#1144): `scorsese import` takes an `.html`
-- page, and the web refused one with 415, so a page made elsewhere could be
-- imported locally but not uploaded. A page is kept as kind `html`; importing
-- it copies its text into the project's `pages/` (project_files), so no
-- project refers to the library's copy. Widening the CHECK is all the table
-- needs: no row changes.
ALTER TABLE library_items DROP CONSTRAINT library_items_kind_check;
ALTER TABLE library_items
    ADD CONSTRAINT library_items_kind_check CHECK (kind IN ('video', 'image', 'audio', 'html'));

ALTER TABLE uploads DROP CONSTRAINT uploads_kind_check;
ALTER TABLE uploads
    ADD CONSTRAINT uploads_kind_check CHECK (kind IN ('video', 'image', 'audio', 'html'));
