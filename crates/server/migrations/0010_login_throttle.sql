-- The login's brake (#557): how many recent login attempts each email and each
-- client address has made, and which are locked out and until when. The rules
-- are crates/server/src/accounts/throttle.rs's; docs/web.md (Accounts) has the
-- argument for keeping them here rather than in the server's memory.
--
-- Not per user, deliberately, and listed in tests/isolation.rs's NOT_PER_USER
-- with the reason: a login attempt belongs to nobody yet. Its email may be
-- nobody's account at all — which is the point, since an unknown email must
-- be braked exactly like a real one — and an address is not an account.
-- Only the login and the operator's commands touch it, privileged; a member
-- may not read it, since whose email is being guessed at is nobody's business
-- but the operator's.
CREATE TABLE login_throttle (
    -- 'email' (normalised as the login normalises it) or 'address' (an IPv4
    -- address, or the /64 an IPv6 address is in).
    kind         TEXT NOT NULL CHECK (kind IN ('email', 'address')),
    key          TEXT NOT NULL CHECK (btrim(key) <> '' AND length(key) <= 254),
    -- Attempts counted since window_start; each is counted before its
    -- password is checked, and a success gives it back.
    attempts     INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    window_start BIGINT NOT NULL,
    -- Seconds since the epoch; 0 when not locked.
    locked_until BIGINT NOT NULL DEFAULT 0,
    -- How many lock-outs in a row, which is what doubles the next one.
    strikes      INTEGER NOT NULL DEFAULT 0 CHECK (strikes >= 0),
    -- The last attempt, for forgiving strikes after a quiet day and for
    -- forgetting the row altogether.
    touched      BIGINT NOT NULL,
    PRIMARY KEY (kind, key)
);
CREATE INDEX login_throttle_touched ON login_throttle (touched);
REVOKE ALL ON login_throttle FROM scorsese_member;
