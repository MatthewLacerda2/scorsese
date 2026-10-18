-- Every answer a turn's quotes were given, in order (#1440): what the chat
-- panel shows once a box is answered. A turn can quote, hear a change asked
-- for, and quote again, and the one `quote` column holds only the latest; the
-- person's own words on a change or a message written instead stay here.
-- Each entry is {answer: confirmed | declined | changed | withdrawn, micros,
-- words}; `quote_answer` stays the latest quote's answer, as before.
ALTER TABLE chat_turns ADD COLUMN quote_replies JSONB NOT NULL DEFAULT '[]';

UPDATE chat_turns
SET quote_replies = jsonb_build_array(jsonb_build_object(
        'answer', quote_answer, 'micros', (quote ->> 'micros')::bigint, 'words', NULL))
WHERE quote_answer IS NOT NULL AND quote IS NOT NULL;
