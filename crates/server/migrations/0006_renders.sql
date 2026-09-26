-- Finished renders (#541): one row per file in the render cache, keyed by a
-- hash of the project's document and the render settings. The argument is in
-- crates/server/src/renders/mod.rs and docs/web.md (Renders).
--
-- Per-user like every other table (crates/server/src/db/scope.rs). Eviction is
-- the one cross-user thing done to it -- the quota is the whole machine's --
-- and runs privileged.
--
-- A row promises a file and nothing more: deleting one, or its file, is always
-- safe, because the render can be made again from its stored project.
CREATE TABLE renders (
    id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id      BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    project_id   BIGINT NOT NULL,
    -- SHA-256 of the document and the settings: the same edit asked for in
    -- the same shape is the same render.
    key          TEXT NOT NULL CHECK (key ~ '^[0-9a-f]{64}$'),
    -- Container, codecs and resolution, every default filled in.
    settings     JSONB NOT NULL CHECK (jsonb_typeof(settings) = 'object'),
    -- Where the file is, relative to the render cache's root: no absolute
    -- paths, so the cache can move without a row changing.
    path         TEXT NOT NULL UNIQUE CHECK (path <> '' AND left(path, 1) <> '/'),
    size         BIGINT NOT NULL CHECK (size >= 0),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- A download, or asking for a render that is already here. Eviction takes
    -- what has not been used in 48 hours.
    last_used_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (project_id, key),
    -- A render goes with its project, and never points at another user's.
    FOREIGN KEY (project_id, user_id) REFERENCES projects (id, user_id) ON DELETE CASCADE
);
CREATE INDEX renders_user_id ON renders (user_id);
-- What eviction reads: everything idle, oldest first.
CREATE INDEX renders_last_used_at ON renders (last_used_at);
ALTER TABLE renders ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON renders USING (user_id = (SELECT member_id()));
