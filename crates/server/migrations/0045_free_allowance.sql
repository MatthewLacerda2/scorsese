-- The free allowance (#1530): every account may spend so much a day, capped
-- so much a calendar month, both in Brasília time, before its paid balance.
-- The argument is in crates/server/src/credits/allowance.rs.
--
-- The allowance is never deposited. When a spend is written, the part the
-- allowance covers is written beside it as an 'allowance' entry: positive,
-- settling the reservation or the assistant's charge it paid for. A
-- generation that fails gives that part back with a negative 'allowance'
-- entry settling the same reservation, so its free part returns to the
-- allowance and never to the paid balance. Nothing unspent is granted, so
-- nothing expires, and the balance is still the sum of the entries.

-- What each account is given, as the server was last started with
-- (SCORSESE_FREE_DAILY, SCORSESE_FREE_MONTHLY). One row, the operator's;
-- members read it and write nothing. Nothing until a server says otherwise.
CREATE TABLE free_allowance (
    one            BOOLEAN PRIMARY KEY DEFAULT true CHECK (one),
    daily_micros   BIGINT NOT NULL CHECK (daily_micros >= 0),
    monthly_micros BIGINT NOT NULL CHECK (monthly_micros >= 0)
);
INSERT INTO free_allowance (daily_micros, monthly_micros) VALUES (0, 0);
REVOKE INSERT, UPDATE, DELETE ON free_allowance FROM scorsese_member;

ALTER TABLE credit_entries
    DROP CONSTRAINT credit_entries_kind,
    DROP CONSTRAINT credit_entries_sign,
    ADD CONSTRAINT credit_entries_kind CHECK (kind IN
        ('top_up', 'reservation', 'release', 'charge', 'refund', 'allowance')),
    ADD CONSTRAINT credit_entries_sign CHECK (CASE kind
        WHEN 'top_up'      THEN amount_micros > 0
        WHEN 'release'     THEN amount_micros > 0
        WHEN 'refund'      THEN amount_micros > 0
        WHEN 'reservation' THEN amount_micros < 0
        WHEN 'allowance'   THEN amount_micros <> 0
        ELSE amount_micros <= 0 END),
    -- An allowance always pays for the spend it settles.
    ADD CONSTRAINT credit_entries_allowance_settles
        CHECK (kind <> 'allowance' OR settles IS NOT NULL),
    -- A Veo shot is paid from the paid balance only, whoever asks for it.
    ADD CONSTRAINT credit_entries_allowance_never_veo
        CHECK (kind <> 'allowance' OR veo_generation_id IS NULL);

-- 0004's check that only a release or a charge settles anything, named by
-- Postgres; dropped by what it says and written again with the allowance.
DO $$
DECLARE
    stale RECORD;
BEGIN
    FOR stale IN
        SELECT conname FROM pg_constraint
        WHERE conrelid = 'credit_entries'::regclass AND contype = 'c'
          AND pg_get_constraintdef(oid) LIKE '%settles IS NULL%'
          AND conname <> 'credit_entries_allowance_settles'
    LOOP
        EXECUTE format('ALTER TABLE credit_entries DROP CONSTRAINT %I', stale.conname);
    END LOOP;
END $$;
ALTER TABLE credit_entries ADD CONSTRAINT credit_entries_what_settles
    CHECK (kind IN ('release', 'charge', 'allowance') OR settles IS NULL);

-- One grant per spend, and one return per grant: free credit is given back
-- at most once, so a failure cannot be counted twice.
CREATE UNIQUE INDEX credit_entries_one_allowance ON credit_entries (settles)
    WHERE kind = 'allowance' AND amount_micros > 0;
CREATE UNIQUE INDEX credit_entries_one_allowance_back ON credit_entries (settles)
    WHERE kind = 'allowance' AND amount_micros < 0;
