-- A user's projects: each edit as its whole project.json document, and which
-- of the user's files each one uses. The argument is in
-- crates/server/src/projects/mod.rs and docs/web.md (Projects).

-- One project. The document is exactly what a .scor folder's project.json
-- holds, loaded into scorsese_core::Project and edited by the same functions
-- the CLI and MCP use; the timeline is deliberately not normalised.
--
-- `name` is generated from the document rather than stored beside it, so a
-- list of projects need not read every document and the two can never
-- disagree. `revision` is the conflict rule: a save names the revision it was
-- based on and is refused when it is no longer the current one.
--
-- UNIQUE (id, user_id) exists for project_assets' foreign key, which is what
-- stops a row there pointing at another user's project.
CREATE TABLE projects (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    document   JSONB NOT NULL CHECK (jsonb_typeof(document) = 'object'),
    name       TEXT GENERATED ALWAYS AS (document ->> 'name') STORED,
    revision   BIGINT NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (id, user_id)
);
CREATE INDEX projects_user_id ON projects (user_id);
ALTER TABLE projects ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON projects USING (user_id = (SELECT member_id()));

-- Which files a project uses, by content: one row per distinct sha256 in its
-- assets table. Derived from the document on every save and never written any
-- other way, so it cannot drift from what the document says.
--
-- A file is named by (user_id, sha256) because that is what identifies one in
-- the library (#535): stored once per user and hash. The foreign key to the
-- library's table is #535's to add, with the table.
CREATE TABLE project_assets (
    project_id BIGINT NOT NULL,
    user_id    BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    sha256     TEXT NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    PRIMARY KEY (project_id, sha256),
    FOREIGN KEY (project_id, user_id) REFERENCES projects (id, user_id) ON DELETE CASCADE
);
CREATE INDEX project_assets_file ON project_assets (user_id, sha256);
ALTER TABLE project_assets ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON project_assets USING (user_id = (SELECT member_id()));
