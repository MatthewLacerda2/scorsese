-- A turn that compacted its conversation (#1438): it opened past 150K tokens,
-- so its own first messages carry a summary of every turn before it, and the
-- next turn's history starts at it and replays nothing older. The older turns
-- stay as they were sent; only where the history begins moves. A flag, not
-- the summary's text: the summary is already in this turn's `messages` and
-- `record`, as the model was sent it.
ALTER TABLE chat_turns ADD COLUMN compacted BOOLEAN NOT NULL DEFAULT false;
