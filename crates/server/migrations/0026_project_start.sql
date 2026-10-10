-- What a project was started for (#1016): the placement its video is going to
-- and the kind of video it is, chosen in the web app's new-project dialog and
-- changeable from the editor. Both optional, both the ids
-- scorsese_core::style names them by.
--
-- Beside the document, never in it: a platform is a render preset and a style
-- is a prompt, so neither is part of the edit (docs/web.md, *The edit is a
-- document*). What the assistant reads is the brief in the project's script;
-- these columns are only the web's memory of the choice -- what the dialog
-- shows as chosen, and what a render with no size asked for is delivered at.
-- No CHECK on the values: the lists are the code's, and the server refuses an
-- unknown id when it is written, as it does for assistant_model.
ALTER TABLE projects ADD COLUMN platform TEXT;
ALTER TABLE projects ADD COLUMN style TEXT;
