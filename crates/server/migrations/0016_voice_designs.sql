-- Designing a voice on the web (#572): the fourth kind of paid generation,
-- and the record of what made every voice a user kept.
--
-- voice_designs is a design's audit row, as veo_generations is a shot's --
-- reserved, then settled when ElevenLabs answers, a failure free -- and it is
-- also the design itself once it worked: the three candidates, each with the
-- vendor's generated_voice_id and the hash its sample is kept under in the
-- user's library (`sample`). Locally that is generated/voice-design/<hash>/
-- design.json beside the samples; here the samples are library items and the
-- record is this row. A design is found again by brief_hash, so an unchanged
-- description is never paid for twice, in any of the user's projects.
--
-- project_id carries no foreign key, for 0004's reason, and is optional: a
-- voice is the user's, not a project's, and the project only says where the
-- design was asked for.
CREATE TABLE voice_designs (
    id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    project_id      BIGINT,
    tool_call_id    BIGINT,
    -- The first candidate's sample, so the settling query is the same for
    -- every kind of generation; all three are in candidates.
    library_item_id BIGINT,
    job_id          BIGINT REFERENCES jobs (id) ON DELETE SET NULL,
    prompt          TEXT NOT NULL,
    passage         TEXT NOT NULL,
    -- The vendor's seed is a u32, which an INTEGER cannot hold.
    seed            BIGINT CHECK (seed BETWEEN 0 AND 4294967295),
    guidance        DOUBLE PRECISION,
    characters      INTEGER NOT NULL CHECK (characters >= 0),
    brief_hash      TEXT NOT NULL,
    state           TEXT NOT NULL DEFAULT 'pending'
                    CHECK (state IN ('pending', 'generated', 'failed')),
    candidates      JSONB NOT NULL DEFAULT '[]' CHECK (jsonb_typeof(candidates) = 'array'),
    estimated_cost_micros BIGINT NOT NULL CHECK (estimated_cost_micros >= 0),
    error           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at     TIMESTAMPTZ,
    UNIQUE (id, user_id),
    FOREIGN KEY (tool_call_id, user_id) REFERENCES tool_calls (id, user_id),
    CONSTRAINT voice_designs_library_item
        FOREIGN KEY (library_item_id, user_id) REFERENCES library_items (id, user_id)
        ON DELETE SET NULL (library_item_id)
);
CREATE INDEX voice_designs_user_id ON voice_designs (user_id, id DESC);
CREATE INDEX voice_designs_brief ON voice_designs (user_id, brief_hash);
CREATE INDEX voice_designs_library_item ON voice_designs (library_item_id);
ALTER TABLE voice_designs ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON voice_designs USING (user_id = (SELECT member_id()));

-- The ledger names the design an entry paid for, and an entry is about one
-- generation at most.
ALTER TABLE credit_entries
    ADD COLUMN voice_design_id BIGINT REFERENCES voice_designs (id),
    ADD CHECK (voice_design_id IS NULL
               OR (veo_generation_id IS NULL AND speech_generation_id IS NULL
                   AND image_generation_id IS NULL));

-- Every voice a user kept out of a design: what designed-voices.json holds in
-- a .scor folder. The voice lives in the operator's ElevenLabs account, not
-- in any project, so this is the user's -- every project of theirs may name
-- it -- and it is what makes a voice that was later deleted worth asking for
-- again: the design it came from keeps the description and the seed.
-- Append-only by use, never by rule: keeping one candidate twice made two
-- voices, and both are recorded.
CREATE TABLE designed_voices (
    id                 BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id            BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    design_id          BIGINT NOT NULL,
    voice_id           TEXT NOT NULL CHECK (btrim(voice_id) <> ''),
    name               TEXT NOT NULL CHECK (btrim(name) <> ''),
    generated_voice_id TEXT NOT NULL,
    provider           TEXT NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (design_id, user_id) REFERENCES voice_designs (id, user_id)
);
CREATE INDEX designed_voices_user_id ON designed_voices (user_id, id);
ALTER TABLE designed_voices ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON designed_voices USING (user_id = (SELECT member_id()));
