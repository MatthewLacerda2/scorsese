-- Credits (#537): what each user has paid in and spent, and a record of every
-- paid generation. The argument is in crates/server/src/credits/mod.rs and
-- docs/web.md (Credits). Money is integer micro-dollars (BIGINT), never floats.
--
-- Per-user like every other table (crates/server/src/db/scope.rs), except
-- display_rates, which is the operator's and nobody's in particular.

-- A Veo shot somebody paid for, or tried to. Written when the shot is
-- reserved for, finished when the provider answers. Kept when the project it
-- was made for is deleted: it is the record of money moving.
--
-- project_id, tool_call_id and library_item_id carry no foreign key on
-- purpose. A project may be deleted and the charge must still say where it
-- came from; tool_calls (#540) and library_items (#535) do not exist yet, and
-- whichever issue adds them adds the key it can.
CREATE TABLE veo_generations (
    id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    project_id      BIGINT,
    tool_call_id    BIGINT,
    library_item_id BIGINT,
    job_id          BIGINT REFERENCES jobs (id) ON DELETE SET NULL,
    model           TEXT NOT NULL,
    resolution      TEXT NOT NULL,
    seconds         INTEGER NOT NULL CHECK (seconds > 0),
    aspect          TEXT NOT NULL,
    prompt          TEXT NOT NULL,
    brief_hash      TEXT NOT NULL,
    -- The provider's operation ticket: proof the provider accepted, and so
    -- that money was spent.
    ticket          TEXT,
    -- pending -> generated | failed. A shot stuck past the worker's patience
    -- stays pending, its reservation held, until it is collected.
    state           TEXT NOT NULL DEFAULT 'pending'
                    CHECK (state IN ('pending', 'generated', 'failed')),
    -- The provider's price by scorsese's own table, before the markup.
    estimated_cost_micros BIGINT NOT NULL CHECK (estimated_cost_micros >= 0),
    error           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at     TIMESTAMPTZ
);
CREATE INDEX veo_generations_user_id ON veo_generations (user_id, id DESC);
ALTER TABLE veo_generations ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON veo_generations USING (user_id = (SELECT member_id()));

-- A spoken line somebody paid for, or tried to. The same life as a shot.
CREATE TABLE speech_generations (
    id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    project_id      BIGINT,
    tool_call_id    BIGINT,
    library_item_id BIGINT,
    job_id          BIGINT REFERENCES jobs (id) ON DELETE SET NULL,
    model           TEXT NOT NULL,
    voice           TEXT NOT NULL,
    text            TEXT NOT NULL,
    characters      INTEGER NOT NULL CHECK (characters >= 0),
    -- Whatever else the request carried: stability, style, speed…
    settings        JSONB NOT NULL DEFAULT '{}',
    state           TEXT NOT NULL DEFAULT 'pending'
                    CHECK (state IN ('pending', 'generated', 'failed')),
    estimated_cost_micros BIGINT NOT NULL CHECK (estimated_cost_micros >= 0),
    error           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at     TIMESTAMPTZ
);
CREATE INDEX speech_generations_user_id ON speech_generations (user_id, id DESC);
ALTER TABLE speech_generations ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON speech_generations USING (user_id = (SELECT member_id()));

-- The ledger. Append-only: an entry is never edited, and a correction is a
-- new entry. A user's balance is the sum of their entries' amounts.
--
-- Signed amounts: money in is positive (top_up, release, refund), money out
-- negative (reservation, charge, monthly_fee). A paid generation reserves its
-- price first; the provider's answer settles it with a release (+ the
-- reservation) and, if it worked, a charge. So a failed generation nets to
-- zero, and a reservation is settled once: one release each, by index.
CREATE TABLE credit_entries (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id       BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    kind          TEXT NOT NULL CHECK (kind IN
                  ('top_up', 'reservation', 'release', 'charge', 'refund', 'monthly_fee')),
    amount_micros BIGINT NOT NULL,
    -- The reservation a release or charge settles.
    settles       BIGINT REFERENCES credit_entries (id),
    project_id    BIGINT,
    veo_generation_id    BIGINT REFERENCES veo_generations (id),
    speech_generation_id BIGINT REFERENCES speech_generations (id),
    -- The first day of the month of an account's life a monthly fee pays for.
    fee_period    DATE,
    -- What it was, in words for its owner.
    memo          TEXT NOT NULL CHECK (btrim(memo) <> ''),
    -- The figures behind it: reais and rate for a top-up, token counts for an
    -- assistant call.
    detail        JSONB NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (CASE kind
        WHEN 'top_up'      THEN amount_micros > 0
        WHEN 'release'     THEN amount_micros > 0
        WHEN 'refund'      THEN amount_micros > 0
        WHEN 'reservation' THEN amount_micros < 0
        WHEN 'monthly_fee' THEN amount_micros < 0
        ELSE amount_micros <= 0 END),
    CHECK (kind <> 'release' OR settles IS NOT NULL),
    CHECK (kind IN ('release', 'charge') OR settles IS NULL),
    CHECK ((kind = 'monthly_fee') = (fee_period IS NOT NULL)),
    CHECK (veo_generation_id IS NULL OR speech_generation_id IS NULL)
);
CREATE INDEX credit_entries_user_id ON credit_entries (user_id, id);
CREATE UNIQUE INDEX credit_entries_one_release ON credit_entries (settles)
    WHERE kind = 'release';
CREATE UNIQUE INDEX credit_entries_one_charge ON credit_entries (settles)
    WHERE kind = 'charge';
CREATE UNIQUE INDEX credit_entries_one_fee ON credit_entries (user_id, fee_period)
    WHERE kind = 'monthly_fee';
ALTER TABLE credit_entries ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON credit_entries USING (user_id = (SELECT member_id()));

-- Append-only, held by the database and not by whoever writes the next query.
-- A member may not update or delete at all; and nobody may, the login role
-- included, except a delete cascading from the account's own deletion — the
-- one way a user's rows leave (docs/web.md, Accounts). A cascade reaches this
-- trigger from inside the foreign key's own trigger, so its depth is above 1.
REVOKE UPDATE, DELETE ON credit_entries FROM scorsese_member;

CREATE FUNCTION credit_entries_append_only() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' AND pg_trigger_depth() > 1 THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'credit_entries is append-only: % is refused', TG_OP
        USING ERRCODE = 'insufficient_privilege',
              HINT = 'a correction is a new entry (crates/server/src/credits/mod.rs)';
END
$$;
CREATE TRIGGER append_only BEFORE UPDATE OR DELETE ON credit_entries
    FOR EACH ROW EXECUTE FUNCTION credit_entries_append_only();
CREATE TRIGGER append_only_truncate BEFORE TRUNCATE ON credit_entries
    FOR EACH STATEMENT EXECUTE FUNCTION credit_entries_append_only();

-- The USD -> BRL rate balances are shown at, as "≈ R$ …". Set by the operator
-- and dated; the newest row is the rate. Not per user: it is one fact about
-- the world, the same for everyone. Members read it and write nothing.
CREATE TABLE display_rates (
    id             BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    -- Reais per dollar, in ten-thousandths: 5.4321 is 54321.
    brl_per_usd_e4 BIGINT NOT NULL CHECK (brl_per_usd_e4 > 0),
    set_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);
REVOKE INSERT, UPDATE, DELETE ON display_rates FROM scorsese_member;
