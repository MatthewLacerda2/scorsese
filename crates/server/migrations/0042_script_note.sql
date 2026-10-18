-- The note that the person rewrote the script from the editor (#1488). The
-- argument is in crates/server/src/assistant/store/hands.rs.

-- The revision a call's save made, or null when it saved nothing: a call that
-- left the project as it was — a script written back unchanged — moved
-- nothing anyone needs telling about. Rows from before this column read null.
ALTER TABLE tool_calls ADD COLUMN saved_revision BIGINT;

-- When a turn paused on a question or a quote was told, as it resumed, that
-- the person had rewritten the script: the next turn is told only of a
-- rewrite after that, not of the same one again.
ALTER TABLE chat_turns ADD COLUMN script_told_at TIMESTAMPTZ;
