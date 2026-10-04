-- The assistant's model is a choice a project makes (#705): one of the four
-- crates/providers/src/chat/model.rs offers, by the id its vendor's API takes.
-- Every project starts on Gemini 3.8 Flash, the existing ones included; the
-- owner changes it at any time, mid-conversation too. No CHECK on the value:
-- the server refuses an unknown id when it is written, and the list of models
-- is the code's, not the schema's.
ALTER TABLE projects ADD COLUMN assistant_model TEXT NOT NULL DEFAULT 'gemini-3.8-flash';

-- A turn's messages kept a second time, as no vendor in particular writes
-- them (providers::chat::record): what a turn on another model is built from.
-- `messages` stays the exact bytes the turn's own model was sent. NULL on a
-- turn from before #705, whose `messages` are Anthropic's alone and are read
-- back into the record when a later turn needs it.
ALTER TABLE chat_turns ADD COLUMN record TEXT;
