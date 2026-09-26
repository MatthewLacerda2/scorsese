-- The job queue (#536): long work -- renders, Veo shots, spoken lines,
-- thumbnails -- as one row each, claimed by the server's worker and picked up
-- again after a crash. The argument is in crates/server/src/jobs/mod.rs; there
-- is no message broker (#527).
--
-- Per-user like every other table (crates/server/src/db/scope.rs): a user sees
-- and enqueues only their own jobs. Claiming a job and recovering after a crash
-- are cross-user by nature and run privileged; everything after a claim -- a
-- ticket kept, the job finished -- runs scoped as the job's owner.
CREATE TABLE jobs (
    id             BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id        BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    -- Which handler runs it. Declared in code (jobs::kinds), not here, so a new
    -- kind is not a migration; the shape keeps it a name.
    kind           TEXT NOT NULL CHECK (kind ~ '^[a-z][a-z0-9_]*$'),
    -- What to do, in the kind's own terms.
    payload        JSONB NOT NULL DEFAULT '{}',
    -- waiting -> running -> done | failed | stuck. `stuck` is a provider job
    -- that outlasted its patience: not lost, its ticket is still here.
    state          TEXT NOT NULL DEFAULT 'waiting'
                   CHECK (state IN ('waiting', 'running', 'done', 'failed', 'stuck')),
    -- Times claimed. Recovery gives up on a job interrupted this often.
    attempts       INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    -- A provider's operation ticket, written the moment the provider accepts
    -- the work and never cleared: it is the record that money was spent, and
    -- what a recovered job polls instead of submitting again.
    ticket         TEXT,
    ticket_at      TIMESTAMPTZ,
    -- What a finished job produced, in the kind's own terms.
    result         JSONB,
    -- Why it failed or is stuck, in words for its owner.
    error          TEXT,
    -- The last time a crash or a restart cut it off: who a power cut affected.
    interrupted_at TIMESTAMPTZ,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    started_at     TIMESTAMPTZ,
    finished_at    TIMESTAMPTZ,
    CHECK ((ticket IS NULL) = (ticket_at IS NULL))
);
-- What the worker's claim reads: the oldest waiting job of a kind with room.
CREATE INDEX jobs_waiting ON jobs (kind, id) WHERE state = 'waiting';
-- What the fair-share order and recovery read.
CREATE INDEX jobs_running ON jobs (user_id) WHERE state = 'running';
-- A user's list, newest first.
CREATE INDEX jobs_user_id ON jobs (user_id, id DESC);
ALTER TABLE jobs ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON jobs USING (user_id = (SELECT member_id()));
