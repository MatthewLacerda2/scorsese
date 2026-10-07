-- A new project's assistant starts on Claude Sonnet 5.5 (the maintainer,
-- 2026-10-07: the balance of quality and price), where 0019 started it on
-- Gemini 3.8 Flash. The code names the default when it creates a project
-- (scorsese_providers::chat::Model::DEFAULT), so this only keeps the column
-- from saying otherwise. Existing projects keep the model they are on: the
-- column cannot tell a model somebody chose from the one they were given.
ALTER TABLE projects ALTER COLUMN assistant_model SET DEFAULT 'claude-sonnet-5-5';
