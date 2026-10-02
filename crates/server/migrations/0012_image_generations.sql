-- Generated stills (#461): a Gemini image somebody paid for, or tried to. The
-- same life as a shot in 0004 -- reserved, then settled when the provider
-- answers -- minus the ticket: a picture comes back on the call that asked
-- for it, so there is never one to keep.
--
-- project_id carries no foreign key, for 0004's reason: a project may be
-- deleted and the charge must still say where it came from. tool_call_id and
-- library_item_id carry theirs from the start, since both tables exist now.
CREATE TABLE image_generations (
    id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    project_id      BIGINT,
    tool_call_id    BIGINT,
    library_item_id BIGINT,
    job_id          BIGINT REFERENCES jobs (id) ON DELETE SET NULL,
    model           TEXT NOT NULL,
    resolution      TEXT NOT NULL,
    aspect          TEXT NOT NULL,
    -- How many reference pictures went with the prompt. Their bytes are in
    -- the brief hash, and the pictures themselves in the user's library.
    references_sent INTEGER NOT NULL CHECK (references_sent >= 0),
    prompt          TEXT NOT NULL,
    brief_hash      TEXT NOT NULL,
    state           TEXT NOT NULL DEFAULT 'pending'
                    CHECK (state IN ('pending', 'generated', 'failed')),
    estimated_cost_micros BIGINT NOT NULL CHECK (estimated_cost_micros >= 0),
    error           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at     TIMESTAMPTZ,
    FOREIGN KEY (tool_call_id, user_id) REFERENCES tool_calls (id, user_id),
    CONSTRAINT image_generations_library_item
        FOREIGN KEY (library_item_id, user_id) REFERENCES library_items (id, user_id)
        ON DELETE SET NULL (library_item_id)
);
CREATE INDEX image_generations_user_id ON image_generations (user_id, id DESC);
CREATE INDEX image_generations_library_item ON image_generations (library_item_id);
ALTER TABLE image_generations ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON image_generations USING (user_id = (SELECT member_id()));

-- The ledger names the still an entry paid for, as it names a shot or a line
-- -- and an entry is about one generation at most, which 0004's check holds
-- for the other two and this one holds for the third.
ALTER TABLE credit_entries
    ADD COLUMN image_generation_id BIGINT REFERENCES image_generations (id),
    ADD CHECK (image_generation_id IS NULL
               OR (veo_generation_id IS NULL AND speech_generation_id IS NULL));
