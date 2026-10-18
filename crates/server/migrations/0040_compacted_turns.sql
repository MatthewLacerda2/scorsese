-- A turn that compacted its conversation (#1438): it opened past 150K tokens,
-- so its own first messages carry a summary of every turn before it, and the
-- next turn's history starts at it and replays nothing older. The older turns
-- stay as they were sent; only where the history begins moves. A flag, not
-- the summary's text: the summary is already in this turn's `messages` and
-- `record`, as the model was sent it.
ALTER TABLE chat_turns ADD COLUMN compacted BOOLEAN NOT NULL DEFAULT false;

-- A call that summarised the conversation rather than carried on the edit.
-- It read the whole of the old conversation, so it says nothing about how big
-- the compacted one is: the size a new turn compares with the threshold skips
-- it, and a turn whose first real call failed after a summary does not
-- compact again.
ALTER TABLE model_calls ADD COLUMN summary BOOLEAN NOT NULL DEFAULT false;
