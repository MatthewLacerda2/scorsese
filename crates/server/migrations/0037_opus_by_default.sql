-- A new project's assistant starts on Claude Opus 5.5 (the maintainer,
-- 2026-10-17, #1444: its cost is worth what it does), where 0022 started it
-- on Claude Sonnet 5.5. The code names the default when it creates a project
-- (scorsese_providers::chat::Model::DEFAULT), so this only keeps the column
-- from saying otherwise. Existing projects keep the model they are on: the
-- column cannot tell a model somebody chose from the one they were given.
ALTER TABLE projects ALTER COLUMN assistant_model SET DEFAULT 'claude-opus-5-5';
