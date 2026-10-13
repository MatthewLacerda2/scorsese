-- Sign in with Google (#952): an account is a Google identity, and passwords
-- are gone.
--
-- `google_sub` is Google's id for the person, "unique among all Google
-- Accounts and never reused": once an account has one, that is what it is
-- known by, never its email. NULL until the account's first Google sign-in,
-- which links it by its verified email (accounts made before this, or by the
-- operator's `user create`), or until the operator links it by hand.
ALTER TABLE users ADD COLUMN google_sub TEXT UNIQUE
    CHECK (google_sub IS NULL OR btrim(google_sub) <> '');

-- Nobody types a password any more, so nothing is hashed, checked or braked.
-- The hashes are dropped rather than kept "just in case": a column of
-- password hashes nobody can use is only something a backup can leak.
ALTER TABLE users DROP COLUMN password_hash;
DROP TABLE login_throttle;
