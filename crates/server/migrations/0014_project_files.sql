-- The authored text a stored project keeps beside its document (#560): its
-- synthesis recipes and its script. A .scor folder holds them as files next
-- to project.json; a stored project held only the document, so on the web
-- there was nothing to bake a recipe from and no script to read. The argument
-- is in crates/server/src/projects/files.rs and docs/web.md (Projects).
--
-- One row per file, keyed by its project-relative path -- `recipes/theme.json`,
-- `script.md` -- holding its text. Text, not bytes: a recipe is JSON and a
-- script is prose, and which paths are kept at all is decided in Rust
-- (projects::files::kept), so the table only refuses what no project-relative
-- path can be. The size cap is the same one the code holds every file to.
CREATE TABLE project_files (
    project_id BIGINT NOT NULL,
    user_id    BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    path       TEXT NOT NULL CHECK (
                   path <> '' AND left(path, 1) <> '/' AND path !~ '(^|/)\.\.(/|$)'
               ),
    content    TEXT NOT NULL CHECK (octet_length(content) <= 1048576),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, path),
    FOREIGN KEY (project_id, user_id) REFERENCES projects (id, user_id) ON DELETE CASCADE
);
ALTER TABLE project_files ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON project_files USING (user_id = (SELECT member_id()));
