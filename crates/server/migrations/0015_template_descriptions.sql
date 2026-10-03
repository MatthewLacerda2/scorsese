-- What a template is for (#560): a line of prose of its own, so a person or
-- an assistant can tell an intro from an outro before inserting one. Beside
-- the document rather than in it: a template document is a project.json
-- document, and a field in it would be a format change (schema bump and
-- migration) for something no project has. The argument is in
-- crates/server/src/templates/mod.rs and scorsese_core::template::Description.
ALTER TABLE templates ADD COLUMN description TEXT
    CHECK (description IS NULL OR char_length(description) BETWEEN 1 AND 2000);
