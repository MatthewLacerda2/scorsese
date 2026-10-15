-- The access record Marco Civil da Internet art. 15 asks of an application
-- run by a company for profit (#1267): the address, date and time of each
-- access, kept at least six months. The rules for writing it are
-- crates/server/src/accounts/access.rs's; docs/web.md (Accounts, *The access
-- record*) has how the operator reads it.
--
-- Not per user, deliberately, and listed in tests/isolation.rs's NOT_PER_USER
-- with the reason: the law says the record outlives the account, so
-- `user_id` is a plain number with no foreign key and no cascade, and a
-- deleted account leaves its rows here. Account ids are never reused, so a
-- row keeps naming the same account. Only the server writes it, privileged;
-- a member may not read it, their own rows included, since it is a record
-- kept for the law and not part of the service.
--
-- Never pruned (the maintainer, 2026-10-14): six months is the law's minimum,
-- not a ceiling, and a row a day per active account is nothing to keep.
CREATE TABLE access_log (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id    BIGINT NOT NULL,
    -- 'sign_in': a session was opened by signing in. 'day': the account's
    -- first request of `day`, by session or by API token.
    kind       TEXT NOT NULL CHECK (kind IN ('sign_in', 'day')),
    -- The visitor's address, as the deploy's nginx names it; NULL where it
    -- could not be told (a server reached without nginx or a peer address).
    address    INET,
    at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- `at`'s date in UTC, which is what "first request of the day" counts in.
    day        DATE NOT NULL DEFAULT (now() AT TIME ZONE 'UTC')::date
);
CREATE UNIQUE INDEX access_log_one_day ON access_log (user_id, day) WHERE kind = 'day';
CREATE INDEX access_log_user_at ON access_log (user_id, at);
CREATE INDEX access_log_address_at ON access_log (address, at);
REVOKE ALL ON access_log FROM scorsese_member;
