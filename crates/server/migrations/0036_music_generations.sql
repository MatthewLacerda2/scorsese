-- Generated music (#1264): an ElevenLabs Music track somebody paid for, or
-- tried to. The same life as a line in 0004 -- reserved, then settled when
-- the vendor answers, a failure free -- and like a line it has no ticket: the
-- track comes back on the call that asked for it.
--
-- project_id carries no foreign key, for 0004's reason: a project may be
-- deleted and the charge must still say where it came from.
CREATE TABLE music_generations (
    id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    project_id      BIGINT,
    tool_call_id    BIGINT,
    library_item_id BIGINT,
    job_id          BIGINT REFERENCES jobs (id) ON DELETE SET NULL,
    model           TEXT NOT NULL,
    prompt          TEXT NOT NULL,
    -- How long the track was asked to be: what it is priced by.
    seconds         INTEGER NOT NULL CHECK (seconds > 0),
    instrumental    BOOLEAN NOT NULL,
    brief_hash      TEXT NOT NULL,
    state           TEXT NOT NULL DEFAULT 'pending'
                    CHECK (state IN ('pending', 'generated', 'failed')),
    estimated_cost_micros BIGINT NOT NULL CHECK (estimated_cost_micros >= 0),
    error           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at     TIMESTAMPTZ,
    FOREIGN KEY (tool_call_id, user_id) REFERENCES tool_calls (id, user_id),
    CONSTRAINT music_generations_library_item
        FOREIGN KEY (library_item_id, user_id) REFERENCES library_items (id, user_id)
        ON DELETE SET NULL (library_item_id)
);
CREATE INDEX music_generations_user_id ON music_generations (user_id, id DESC);
CREATE INDEX music_generations_library_item ON music_generations (library_item_id);
ALTER TABLE music_generations ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON music_generations USING (user_id = (SELECT member_id()));

-- The ledger names the track an entry paid for, and an entry is about one
-- generation at most.
ALTER TABLE credit_entries
    ADD COLUMN music_generation_id BIGINT REFERENCES music_generations (id),
    ADD CHECK (music_generation_id IS NULL
               OR (veo_generation_id IS NULL AND speech_generation_id IS NULL
                   AND image_generation_id IS NULL AND voice_design_id IS NULL));
