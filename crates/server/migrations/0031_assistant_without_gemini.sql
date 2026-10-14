-- Gemini is no longer a model the assistant offers (#1220, the maintainer,
-- 2026-10-12: not good enough at the job). 0019 put every project of its day
-- on gemini-3.8-flash and 0022 changed only the default, so many projects
-- still name a Gemini model; the server no longer knows one, and would refuse
-- to open their chat. They move to the default, Claude Sonnet 5.5.
UPDATE projects SET assistant_model = 'claude-sonnet-5-5'
WHERE assistant_model LIKE 'gemini-%';

-- A turn paused on a question would resume on its own stored model with its
-- own Gemini-format messages, so it is set aside the way a new conversation
-- sets one aside (store/asking.rs, set_aside_in, with its SET_ASIDE words).
-- Its history stays: the neutral record carries it into the next turn on
-- Claude. Turns still running at deploy are the restart's recover to end.
UPDATE chat_turns
SET state = 'stopped', answer = 'Stopped while waiting for your answer.', finished_at = now()
WHERE state = 'asking' AND model LIKE 'gemini-%';
