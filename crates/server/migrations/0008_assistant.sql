-- The assistant (#540): Claude editing a user's project through scorsese's
-- tools, and the record of every turn it took, every tool it called and every
-- call it was charged for. The argument is in crates/server/src/assistant/
-- and docs/web.md (The assistant).
--
-- Per-user like every other table (crates/server/src/db/scope.rs).

-- A conversation about one project. A project has as many as its owner
-- started; the newest is the one a new turn joins.
--
-- project_id has no foreign key on purpose, as in tool_calls: a project can
-- be deleted, and the record of what was said and spent about it cannot.
CREATE TABLE chat_sessions (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    project_id  BIGINT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (id, user_id)
);
CREATE INDEX chat_sessions_project ON chat_sessions (user_id, project_id, id DESC);
ALTER TABLE chat_sessions ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON chat_sessions USING (user_id = (SELECT member_id()));

-- One prompt and everything the assistant did with it.
CREATE TABLE chat_turns (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id       BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    session_id    BIGINT NOT NULL,
    -- What the user wrote.
    prompt        TEXT NOT NULL CHECK (btrim(prompt) <> ''),
    -- The Messages API messages this turn added to the conversation, as the
    -- exact JSON text first sent: a JSON array, appended to and never
    -- rewritten. TEXT rather than JSONB on purpose — JSONB reorders keys, and
    -- a replayed message must be byte-for-byte what the model saw or it loses
    -- its thinking and misses the prompt cache.
    messages      TEXT NOT NULL DEFAULT '[]',
    -- running -> answered | refused | capped | stopped | failed | interrupted.
    state         TEXT NOT NULL DEFAULT 'running' CHECK (state IN
                  ('running', 'answered', 'refused', 'capped', 'stopped', 'failed', 'interrupted')),
    -- The final answer, or why there is none.
    answer        TEXT,
    -- The API's stop_reason for the last call.
    stop_reason   TEXT,
    model         TEXT NOT NULL,
    effort        TEXT NOT NULL,
    -- Totals over the turn's calls to the model; each call's own figures are
    -- on its credit_entries row.
    calls               INTEGER NOT NULL DEFAULT 0,
    input_tokens        BIGINT NOT NULL DEFAULT 0,
    output_tokens       BIGINT NOT NULL DEFAULT 0,
    cache_write_tokens  BIGINT NOT NULL DEFAULT 0,
    cache_read_tokens   BIGINT NOT NULL DEFAULT 0,
    charged_micros      BIGINT NOT NULL DEFAULT 0 CHECK (charged_micros >= 0),
    -- A paid tool's quote waiting for the user's yes or no (#538): its token,
    -- which the model never sees, and what the confirmation box shows.
    quote_token   TEXT,
    quote         JSONB,
    quote_answer  TEXT CHECK (quote_answer IN ('confirmed', 'declined', 'withdrawn')),
    started_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at   TIMESTAMPTZ,
    UNIQUE (id, user_id),
    FOREIGN KEY (session_id, user_id) REFERENCES chat_sessions (id, user_id) ON DELETE CASCADE
);
CREATE INDEX chat_turns_session ON chat_turns (session_id, id);
-- One turn at a time per conversation: the next one's history is this one.
CREATE UNIQUE INDEX chat_turns_one_running ON chat_turns (session_id) WHERE state = 'running';
ALTER TABLE chat_turns ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON chat_turns USING (user_id = (SELECT member_id()));

-- A tool call names the turn it was made in, and its place there; so does the
-- user's own yes to a quote, which is a call of its own ('user').
ALTER TABLE tool_calls DROP CONSTRAINT tool_calls_client_check;
ALTER TABLE tool_calls
    ADD CONSTRAINT tool_calls_client_check CHECK (client IN ('external', 'assistant', 'user')),
    ADD COLUMN turn_id  BIGINT,
    ADD COLUMN position INTEGER,
    ADD FOREIGN KEY (turn_id, user_id) REFERENCES chat_turns (id, user_id),
    ADD CHECK ((client = 'external') = (turn_id IS NULL)),
    ADD CHECK ((turn_id IS NULL) = (position IS NULL));
CREATE INDEX tool_calls_turn ON tool_calls (turn_id, position) WHERE turn_id IS NOT NULL;

-- An assistant call's charge names its turn, so prompt -> turn -> call ->
-- credits can be read back, and the history shows a turn as one row.
ALTER TABLE credit_entries
    ADD COLUMN chat_turn_id BIGINT,
    ADD FOREIGN KEY (chat_turn_id, user_id) REFERENCES chat_turns (id, user_id);

-- A quote names the tool call that issued it, which is how the assistant
-- finds the token to hold back from the model.
ALTER TABLE quotes ADD COLUMN tool_call_id BIGINT;
