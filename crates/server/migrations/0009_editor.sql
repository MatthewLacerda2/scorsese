-- The web editor (#545): the user's own edits by hand — a clip dragged,
-- trimmed or set in the inspector, a file dropped onto a track — go through
-- the same tools as the assistant's and web MCP's, and are recorded in the
-- same table as client 'editor'. The argument is in
-- crates/server/src/http/editor.rs and docs/web.md (The editor).

ALTER TABLE tool_calls DROP CONSTRAINT tool_calls_client_check;
ALTER TABLE tool_calls
    ADD CONSTRAINT tool_calls_client_check
        CHECK (client IN ('external', 'assistant', 'user', 'editor'));

-- 0008 tied "has no chat turn" to "is web MCP's"; an editor call has no turn
-- either. That check was added unnamed, so Postgres named it tool_calls_check.
ALTER TABLE tool_calls DROP CONSTRAINT tool_calls_check;
ALTER TABLE tool_calls
    ADD CONSTRAINT tool_calls_turn_check
        CHECK ((client IN ('external', 'editor')) = (turn_id IS NULL));
