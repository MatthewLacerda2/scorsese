-- Charged per generation only (#1243, #1247): there is no monthly fee, and a
-- generation is priced by what kind it is, so a quote keeps what it charges.

-- The monthly fee goes, as a kind of entry and as the column only it filled.
-- The two monthly_fee rows the hosted server holds were fictitious: the
-- maintainer funded test accounts out of one card (2026-10-13). They are
-- deleted rather than offset by a refund, since nothing real was paid.
-- credit_entries is append-only and its trigger refuses a DELETE from
-- everybody (0004); this one rewrite of history is the schema's own, made
-- once, in this transaction, so the trigger is lifted around it and restored.
ALTER TABLE credit_entries DISABLE TRIGGER append_only;
DELETE FROM credit_entries WHERE kind = 'monthly_fee';
ALTER TABLE credit_entries ENABLE TRIGGER append_only;

-- Dropping fee_period drops what is built on it: the one-fee-a-month index
-- and the check tying it to the kind.
ALTER TABLE credit_entries DROP COLUMN fee_period;

-- 0004's checks named the kind by its spelling; Postgres named the checks.
-- Every one still mentioning it goes, by what it says rather than by a
-- generated name, and is written again without it.
DO $$
DECLARE
    stale RECORD;
BEGIN
    FOR stale IN
        SELECT conname FROM pg_constraint
        WHERE conrelid = 'credit_entries'::regclass AND contype = 'c'
          AND pg_get_constraintdef(oid) LIKE '%monthly_fee%'
    LOOP
        EXECUTE format('ALTER TABLE credit_entries DROP CONSTRAINT %I', stale.conname);
    END LOOP;
END $$;
ALTER TABLE credit_entries
    ADD CONSTRAINT credit_entries_kind CHECK (kind IN
        ('top_up', 'reservation', 'release', 'charge', 'refund')),
    ADD CONSTRAINT credit_entries_sign CHECK (CASE kind
        WHEN 'top_up'      THEN amount_micros > 0
        WHEN 'release'     THEN amount_micros > 0
        WHEN 'refund'      THEN amount_micros > 0
        WHEN 'reservation' THEN amount_micros < 0
        ELSE amount_micros <= 0 END);

-- A quote keeps what it takes from the balance, in micro-dollars, beside the
-- providers' cents it was quoted at. One quote can hold a shot, a line and a
-- still, each priced by its own kind, so the charge is no longer one sum of
-- the cents times one markup. Quotes live fifteen minutes; any still open
-- across this deploy is withdrawn, and asking again re-quotes it.
DELETE FROM quotes;
ALTER TABLE quotes ADD COLUMN micros BIGINT NOT NULL CHECK (micros >= 0);
