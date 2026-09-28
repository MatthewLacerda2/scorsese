-- Web MCP (#539): every tool call a user's client makes, and the quotes a paid
-- tool is waiting on a yes for. The argument is in crates/server/src/tools/
-- and docs/web.md (Web MCP).
--
-- Per-user like every other table (crates/server/src/db/scope.rs).

-- One row per tool call, whoever made it: a user's own MCP client
-- ('external'), or the built-in assistant (#540, 'assistant') — one table, so
-- "what did anything do to my projects?" has one answer. Written when the
-- call starts and finished when it answers, so a call that takes the server
-- down with it is still on record, unfinished.
--
-- project_id has no foreign key on purpose: a project can be deleted, and the
-- record of what was done to it, and what that cost, cannot.
CREATE TABLE tool_calls (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    client      TEXT NOT NULL CHECK (client IN ('external', 'assistant')),
    tool        TEXT NOT NULL CHECK (btrim(tool) <> ''),
    project_id  BIGINT,
    arguments   JSONB NOT NULL,
    -- null while it runs; then whether it did what was asked or refused.
    outcome     TEXT CHECK (outcome IN ('answered', 'refused')),
    -- The words it answered with. Pictures are not kept: they are the edit's
    -- or the footage's, and either can be drawn again.
    reply       TEXT,
    started_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at TIMESTAMPTZ,
    UNIQUE (id, user_id)
);
CREATE INDEX tool_calls_user_id ON tool_calls (user_id, id DESC);
ALTER TABLE tool_calls ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON tool_calls USING (user_id = (SELECT member_id()));

-- The keys 0004 left for this table to add: a paid generation names the tool
-- call that asked for it, and never another user's.
ALTER TABLE veo_generations
    ADD FOREIGN KEY (tool_call_id, user_id) REFERENCES tool_calls (id, user_id);
ALTER TABLE speech_generations
    ADD FOREIGN KEY (tool_call_id, user_id) REFERENCES tool_calls (id, user_id);

-- A quote a paid tool answered with, waiting for the call that confirms it
-- (#538). The rules — bound to what was quoted, good once, for fifteen
-- minutes — are scorsese_providers::quote's; this is only where the hosted
-- server keeps the record, in place of a local project's cache/quotes/. A row
-- rather than memory, so a restart between the quote and the yes does not
-- turn the yes into a refusal. Taking a token is deleting its row: whoever
-- deletes it spends it, so it is spent at most once.
CREATE TABLE quotes (
    token       TEXT PRIMARY KEY CHECK (token ~ '^quote-[0-9a-f]{24}$'),
    user_id     BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    spend       TEXT NOT NULL,
    digest      TEXT NOT NULL,
    cents       BIGINT NOT NULL CHECK (cents >= 0),
    issued_at   BIGINT NOT NULL,
    expires_at  BIGINT NOT NULL
);
CREATE INDEX quotes_user_id ON quotes (user_id);
ALTER TABLE quotes ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON quotes USING (user_id = (SELECT member_id()));
