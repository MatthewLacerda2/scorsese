-- What models cost and how they behave, queryable (#707): a typed row per call
-- the assistant makes to a model, the size in tokens of each tool reply, and
-- which MCP client a user's own connection says it is. Recording only —
-- nothing here is shown to anyone; docs/web.md (What is recorded) has the
-- queries it is for.
--
-- Per-user like every other table (crates/server/src/db/scope.rs).

-- An entry a model call can name as its own, never another user's.
ALTER TABLE credit_entries ADD UNIQUE (id, user_id);

-- One row per call the assistant made to a model, the same shape whichever
-- vendor answered: providers::chat::Usage, the per-call record #705 made
-- vendor-neutral. A table of its own rather than typed columns on
-- credit_entries: the ledger is about money and holds every kind of entry, so
-- token columns there would be empty on all but one kind; and this row is
-- what a call *did* (its latency, its stop), which the money does not need.
-- The charge it caused is credit_entry_id, written in the same transaction.
CREATE TABLE model_calls (
    id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    turn_id         BIGINT NOT NULL,
    -- Its place in the turn, from 1.
    position        INTEGER NOT NULL CHECK (position >= 1),
    -- The model it ran on, by the id its vendor's API takes, and the vendor
    -- ('anthropic' or 'google').
    model           TEXT NOT NULL,
    vendor          TEXT NOT NULL,
    -- Input read at full price — the cached part of the prompt is not here.
    input_tokens    BIGINT NOT NULL CHECK (input_tokens >= 0),
    -- Output, thinking included: that is how both vendors bill it.
    output_tokens   BIGINT NOT NULL CHECK (output_tokens >= 0),
    -- How much of output_tokens was thinking, where the vendor says so apart:
    -- Gemini does; Anthropic's usage does not, so NULL on a Claude call.
    thinking_tokens BIGINT CHECK (thinking_tokens >= 0),
    cache_write_5m_tokens BIGINT NOT NULL CHECK (cache_write_5m_tokens >= 0),
    cache_write_1h_tokens BIGINT NOT NULL CHECK (cache_write_1h_tokens >= 0),
    cache_read_tokens     BIGINT NOT NULL CHECK (cache_read_tokens >= 0),
    -- From the request being sent to the whole reply being read, streaming
    -- included, for the attempt that answered (a retried failure is not
    -- counted).
    latency_ms      BIGINT NOT NULL CHECK (latency_ms >= 0),
    -- Why it stopped, as the turn's own stop_reason spells it.
    stop_reason     TEXT NOT NULL,
    -- What the vendor's rates make it, before the markup; what the user was
    -- charged is the credit entry's amount.
    cost_micros     BIGINT NOT NULL CHECK (cost_micros >= 0),
    credit_entry_id BIGINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (id, user_id),
    UNIQUE (turn_id, position),
    FOREIGN KEY (turn_id, user_id) REFERENCES chat_turns (id, user_id),
    FOREIGN KEY (credit_entry_id, user_id) REFERENCES credit_entries (id, user_id)
);
CREATE INDEX model_calls_user_id ON model_calls (user_id, id DESC);
CREATE INDEX model_calls_model ON model_calls (model, created_at);
ALTER TABLE model_calls ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON model_calls USING (user_id = (SELECT member_id()));

-- What a tool's reply costs: nothing when it runs, but its words and pictures
-- are input to the model's next call. An estimate, and named one — text at
-- four bytes a token, each picture at width × height / 750 (Anthropic's
-- published figure) — because a vendor's own count is a network round trip
-- per call, and an external client's tokenizer is not known at all. NULL while
-- the call runs; a refusal's words reach the model too, and are counted.
ALTER TABLE tool_calls ADD COLUMN reply_tokens_estimate BIGINT
    CHECK (reply_tokens_estimate >= 0);

-- Which client an API token connects with: the clientInfo its last MCP
-- initialize named. Web MCP holds no sessions, so the token is the
-- connection; a later initialize overwrites it.
ALTER TABLE api_tokens
    ADD COLUMN client_name    TEXT,
    ADD COLUMN client_version TEXT;

-- And on each external call, as the token said it was when the call was made,
-- so a token later used by another client does not rewrite history. No
-- foreign key on api_token_id, as with project_id: a token can be revoked,
-- and the record of what it did cannot.
ALTER TABLE tool_calls
    ADD COLUMN api_token_id   BIGINT,
    ADD COLUMN client_name    TEXT,
    ADD COLUMN client_version TEXT;
