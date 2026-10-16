-- Cloning a voice on the web (#1263): a voice made from the user's own
-- recordings with ElevenLabs' Instant Voice Cloning, recorded beside the
-- voices they designed rather than in a table of its own.
--
-- Beside, because what designed_voices is *for* is every voice a user made in
-- the operator's ElevenLabs account: designs::owners counts its rows as the
-- account's custom voice slots, and a clone takes one exactly as a kept design
-- does ("Custom voice slots are only used for voices you create with Voice
-- Design or Voice Cloning", elevenlabs.io/docs/eleven-creative/voices/
-- voice-library, read 2026-10-16); it refuses another user's voice by its
-- rows; and forgetting a voice, or deleting an account, deletes its rows'
-- voices at the vendor. A second table would be a second place every one of
-- those had to remember to look.
--
-- So a row says which kind it is. A designed one names the design and the
-- candidate it came from, as before; a cloned one names neither, and instead
-- the library items its recordings were (`recordings`, each {item, sha256}) —
-- what makes a lost voice worth cloning again. Every row already here is a
-- design.
ALTER TABLE designed_voices
    ADD COLUMN kind TEXT NOT NULL DEFAULT 'designed'
        CHECK (kind IN ('designed', 'cloned')),
    ADD COLUMN recordings JSONB NOT NULL DEFAULT '[]'
        CHECK (jsonb_typeof(recordings) = 'array'),
    ALTER COLUMN design_id DROP NOT NULL,
    ALTER COLUMN generated_voice_id DROP NOT NULL,
    ADD CHECK (
        (kind = 'designed' AND design_id IS NOT NULL AND generated_voice_id IS NOT NULL)
        OR (kind = 'cloned' AND design_id IS NULL AND generated_voice_id IS NULL
            AND jsonb_array_length(recordings) > 0)
    );
