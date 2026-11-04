-- Generated sound effects (#1265): an ElevenLabs sound somebody paid for, or
-- tried to. A track's life, from 0036 -- reserved, then settled when the
-- vendor answers, a failure free, no ticket: the sound comes back on the call
-- that asked for it.
--
-- project_id carries no foreign key, for 0004's reason: a project may be
-- deleted and the charge must still say where it came from.
CREATE TABLE sound_generations (
    id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    project_id      BIGINT,
    tool_call_id    BIGINT,
    library_item_id BIGINT,
    job_id          BIGINT REFERENCES jobs (id) ON DELETE SET NULL,
    model           TEXT NOT NULL,
    prompt          TEXT NOT NULL,
    -- How long the sound was asked to be, in milliseconds: what it is priced
    -- by. A sound's length takes fractions of a second, so whole seconds
    -- would not say what was sent.
    millis          INTEGER NOT NULL CHECK (millis > 0),
    -- Whether it was made to loop seamlessly.
    loop            BOOLEAN NOT NULL,
    brief_hash      TEXT NOT NULL,
    state           TEXT NOT NULL DEFAULT 'pending'
                    CHECK (state IN ('pending', 'generated', 'failed')),
    estimated_cost_micros BIGINT NOT NULL CHECK (estimated_cost_micros >= 0),
    error           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at     TIMESTAMPTZ,
    FOREIGN KEY (tool_call_id, user_id) REFERENCES tool_calls (id, user_id),
    CONSTRAINT sound_generations_library_item
        FOREIGN KEY (library_item_id, user_id) REFERENCES library_items (id, user_id)
        ON DELETE SET NULL (library_item_id)
);
CREATE INDEX sound_generations_user_id ON sound_generations (user_id, id DESC);
CREATE INDEX sound_generations_library_item ON sound_generations (library_item_id);
ALTER TABLE sound_generations ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON sound_generations USING (user_id = (SELECT member_id()));

-- The ledger names the sound an entry paid for, and an entry is about one
-- generation at most.
ALTER TABLE credit_entries
    ADD COLUMN sound_generation_id BIGINT REFERENCES sound_generations (id),
    ADD CHECK (sound_generation_id IS NULL
               OR (veo_generation_id IS NULL AND speech_generation_id IS NULL
                   AND image_generation_id IS NULL AND voice_design_id IS NULL
                   AND music_generation_id IS NULL));
