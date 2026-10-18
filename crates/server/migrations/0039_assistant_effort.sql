-- How hard a project's assistant thinks is a choice the project keeps
-- (#1443), beside its model, where it was each browser's until now: one of
-- crates/providers/src/chat/mod.rs's `Effort` levels, by the name Anthropic's
-- API takes (low, medium, high, xhigh). Every project starts on high, the
-- level a message without one ran at, the existing ones included. No CHECK on
-- the value, as for assistant_model: the server refuses an unknown name when
-- it is written, and the levels are the code's.
ALTER TABLE projects ADD COLUMN assistant_effort TEXT NOT NULL DEFAULT 'high';
