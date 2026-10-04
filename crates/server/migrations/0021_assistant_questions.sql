-- The assistant asks the person a question mid-edit (#710).
--
-- A turn that asks pauses in a state of its own, `asking`: not `running`,
-- because nothing is running — no process holds it, nothing is charged, and a
-- restart leaves it exactly as it was — and not ended, because the same turn
-- resumes with the answer. The question itself is not a new kind of row: the
-- call that asked it is the last message the turn kept, and its answer will be
-- that call's result. What is stored here is the card the chat panel draws,
-- one entry per question the turn asked, in order: `{question, options,
-- answer}`, `answer` null while it waits.

ALTER TABLE chat_turns DROP CONSTRAINT chat_turns_state_check;
ALTER TABLE chat_turns ADD CONSTRAINT chat_turns_state_check CHECK (state IN
    ('running', 'asking', 'answered', 'refused', 'capped', 'stopped', 'failed', 'interrupted'));
ALTER TABLE chat_turns ADD COLUMN questions JSONB NOT NULL DEFAULT '[]';
