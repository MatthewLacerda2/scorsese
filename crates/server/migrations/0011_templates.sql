-- A user's templates (#546): pieces of an edit saved to be copied into any of
-- their projects -- an intro, an outro, a running gag. The argument is in
-- crates/server/src/templates/mod.rs and docs/web.md (Templates).

-- One template. The document is a project.json document holding only the
-- saved clips, their tracks and the assets they show (scorsese_core::template),
-- so it is migrated on start exactly as a project's is and read by the same
-- code. `name` is generated from it, as a project's is, and unique per user
-- whatever its case: "use my intro" has to name one template.
CREATE TABLE templates (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    document   JSONB NOT NULL CHECK (jsonb_typeof(document) = 'object'),
    name       TEXT GENERATED ALWAYS AS (document ->> 'name') STORED,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (id, user_id)
);
CREATE UNIQUE INDEX templates_name ON templates (user_id, lower(name));
ALTER TABLE templates ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON templates USING (user_id = (SELECT member_id()));

-- Which library files a template uses: one row per distinct sha256 in its
-- document, rewritten on every save and never any other way -- project_assets'
-- rule, for the same reason. The key to library_items is what makes deleting a
-- file a template uses fail, as deleting one a project uses does: inserting
-- the template later would otherwise name a file that is gone.
CREATE TABLE template_assets (
    template_id BIGINT NOT NULL,
    user_id     BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    sha256      TEXT NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    PRIMARY KEY (template_id, sha256),
    FOREIGN KEY (template_id, user_id) REFERENCES templates (id, user_id) ON DELETE CASCADE,
    FOREIGN KEY (user_id, sha256) REFERENCES library_items (user_id, sha256)
);
CREATE INDEX template_assets_file ON template_assets (user_id, sha256);
ALTER TABLE template_assets ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON template_assets USING (user_id = (SELECT member_id()));
