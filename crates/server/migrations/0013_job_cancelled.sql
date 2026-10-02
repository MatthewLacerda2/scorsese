-- A job its owner stopped (#660): `cancelled`, beside done, failed and stuck.
--
-- Its own state rather than `failed`, because nothing went wrong: the owner
-- asked, and the list should say so rather than show an error they caused.
-- `error` carries how far it got ("cancelled after 412 of 1890 frames"), in
-- the same words a failure's reason would be.
--
-- Only the kinds that may be stopped ever reach it -- renders and previews
-- (crates/server/src/jobs/kinds.rs, STOPPABLE); a paid generation is never
-- cancelled, because the provider bills either way.
ALTER TABLE jobs DROP CONSTRAINT jobs_state_check;
ALTER TABLE jobs ADD CONSTRAINT jobs_state_check
    CHECK (state IN ('waiting', 'running', 'done', 'failed', 'stuck', 'cancelled'));
