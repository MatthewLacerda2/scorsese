-- The person's name, as their Google account gives it (#1266): written at
-- sign-up and refreshed on every Google sign-in, so a name changed at Google
-- follows. NULL until then (accounts made before this get theirs at their
-- next sign-in, with no backfill), and for a Google account that has none.
-- Deleting the account deletes the row, and the name with it.
ALTER TABLE users ADD COLUMN name TEXT
    CHECK (name IS NULL OR btrim(name) <> '');
