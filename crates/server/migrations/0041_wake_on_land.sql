-- When the generations a conversation ordered land, the assistant is woken and
-- carries on (#1441). The argument is in crates/server/src/assistant/wake.rs.

-- A job names the tool call that queued it, when a tool did: the user's yes
-- to a quote (client 'user', with its turn) is how a turn's ordered jobs are
-- found. Nothing else reads it; a render or a thumbnail leaves it null.
ALTER TABLE jobs
    ADD COLUMN tool_call_id BIGINT,
    ADD FOREIGN KEY (tool_call_id, user_id) REFERENCES tool_calls (id, user_id);
CREATE INDEX jobs_tool_call ON jobs (tool_call_id) WHERE tool_call_id IS NOT NULL;

-- `awaiting`: the turn ordered jobs and the model has not yet been told how
-- they ended. Cleared by the turn that tells it: the one the server starts
-- when the last of them lands, or the person's next message.
-- `told_job`: the newest of its jobs the model has been told about, so a
-- turn that orders twice is told about each of its jobs once.
-- `woken`: the turn was started by the server when work landed, not by a
-- message; its prompt is the line the transcript shows in place of one.
ALTER TABLE chat_turns
    ADD COLUMN awaiting BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN told_job BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN woken BOOLEAN NOT NULL DEFAULT false;
CREATE INDEX chat_turns_awaiting ON chat_turns (session_id) WHERE awaiting;
