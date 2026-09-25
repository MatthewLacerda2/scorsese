-- Accounts, and the machinery every per-user table after this one inherits.
--
-- The isolation mechanism is argued in crates/server/src/db/scope.rs and in
-- docs/web.md (Accounts). In one paragraph: every per-user table carries
-- `user_id` and a row-level-security policy that compares it with
-- `member_id()`, the user the current transaction was opened for. The server's
-- connections sit in a role that may read nothing at all, and only a
-- transaction opened by `db::scoped` steps into the role that may -- so a
-- query that forgot its scope is refused by Postgres, loudly, rather than
-- returning somebody else's rows.

-- The two roles. Cluster-wide, so they may already exist: a second database
-- on the same server (every #[sqlx::test] is one) finds them there, possibly
-- mid-creation by a sibling, which is the unique_violation.
DO $$
BEGIN
    CREATE ROLE scorsese_unscoped NOLOGIN;
EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL;
END $$;

DO $$
BEGIN
    CREATE ROLE scorsese_member NOLOGIN;
EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL;
END $$;

-- The login role has to be allowed to become either. A superuser already
-- may; granting is what lets a plain owner with CREATEROLE do the same.
DO $$
BEGIN
    GRANT scorsese_unscoped, scorsese_member TO CURRENT_USER;
EXCEPTION WHEN unique_violation THEN NULL;
END $$;

-- Every table and sequence this role creates from here on is readable and
-- writable by a member -- row-level security then narrows it to their rows.
-- scorsese_unscoped is granted nothing, ever: that is its whole purpose.
ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO scorsese_member;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT USAGE, SELECT ON SEQUENCES TO scorsese_member;

-- The user the current transaction acts for, or an error that says what went
-- wrong. `db::scoped` sets `scorsese.member` for the length of a transaction.
CREATE FUNCTION member_id() RETURNS bigint
LANGUAGE plpgsql STABLE AS $$
DECLARE
    setting text := current_setting('scorsese.member', true);
BEGIN
    IF setting IS NULL OR setting = '' THEN
        RAISE EXCEPTION 'a per-user table was queried outside a user scope'
            USING ERRCODE = 'insufficient_privilege',
                  HINT = 'open the transaction with db::scoped (crates/server/src/db/scope.rs)';
    END IF;
    RETURN setting::bigint;
END
$$;

-- An account. The email is stored normalised (trimmed, lower case), which is
-- what makes UNIQUE mean "one account per address" without an extension.
CREATE TABLE users (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE
                  CHECK (email <> '' AND email = lower(btrim(email))),
    password_hash TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_row ON users USING (id = (SELECT member_id()));

-- A browser's login. Only a SHA-256 of the cookie is kept, so a database
-- backup is not a bag of live sessions.
CREATE TABLE sessions (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX sessions_user_id ON sessions (user_id);
ALTER TABLE sessions ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON sessions USING (user_id = (SELECT member_id()));

-- A bearer token for a client that is not a browser: an MCP client, a script.
-- Hashed for the same reason as a session; shown once, when it is issued.
CREATE TABLE api_tokens (
    id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id      BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name         TEXT NOT NULL CHECK (btrim(name) <> ''),
    token_hash   BYTEA NOT NULL UNIQUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ
);
CREATE INDEX api_tokens_user_id ON api_tokens (user_id);
ALTER TABLE api_tokens ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON api_tokens USING (user_id = (SELECT member_id()));
