-- Bonus credit on larger recharges (#1537): a purchase from R$ 40 credits 5%
-- more, from R$ 80 10% more (the tiers are configuration,
-- SCORSESE_RECHARGE_BONUS). The argument is in
-- crates/server/src/payments/bonus.rs and docs/web.md (Buying credits).
--
-- The bonus is locked on the payment with its rate, so what the person was
-- shown is what they get, and it is credited with the top-up, once, as its
-- own 'bonus' entry linked to the same payment.
--
-- A purchase withdrawn within its 7 days (CDC art. 49) takes back what is
-- left of it, its bonus included, as one 'withdrawal' entry linked to the
-- payment: the operator's `scorsese-server credit withdraw`.
ALTER TABLE payments
    ADD COLUMN bonus_micros BIGINT NOT NULL DEFAULT 0 CHECK (bonus_micros >= 0);

ALTER TABLE credit_entries
    DROP CONSTRAINT credit_entries_kind,
    DROP CONSTRAINT credit_entries_sign,
    ADD CONSTRAINT credit_entries_kind CHECK (kind IN
        ('top_up', 'reservation', 'release', 'charge', 'refund', 'allowance',
         'adjustment', 'bonus', 'withdrawal')),
    ADD CONSTRAINT credit_entries_sign CHECK (CASE kind
        WHEN 'top_up'      THEN amount_micros > 0
        WHEN 'release'     THEN amount_micros > 0
        WHEN 'refund'      THEN amount_micros > 0
        WHEN 'bonus'       THEN amount_micros > 0
        WHEN 'reservation' THEN amount_micros < 0
        WHEN 'adjustment'  THEN amount_micros < 0
        WHEN 'withdrawal'  THEN amount_micros < 0
        WHEN 'allowance'   THEN amount_micros <> 0
        ELSE amount_micros <= 0 END),
    -- A bonus and a withdrawal each belong to a purchase.
    ADD CONSTRAINT credit_entries_payment_needed
        CHECK (kind NOT IN ('bonus', 'withdrawal') OR payment_id IS NOT NULL);

-- 0029's check that only a top-up names a payment, named by Postgres;
-- dropped by what it says and written again with the bonus and withdrawal.
DO $$
DECLARE
    stale RECORD;
BEGIN
    FOR stale IN
        SELECT conname FROM pg_constraint
        WHERE conrelid = 'credit_entries'::regclass AND contype = 'c'
          AND pg_get_constraintdef(oid) LIKE '%payment_id IS NULL%'
    LOOP
        EXECUTE format('ALTER TABLE credit_entries DROP CONSTRAINT %I', stale.conname);
    END LOOP;
END $$;
ALTER TABLE credit_entries ADD CONSTRAINT credit_entries_payment_kinds
    CHECK (payment_id IS NULL OR kind IN ('top_up', 'bonus', 'withdrawal'));

-- One entry of each kind per payment: one top-up, one bonus, one withdrawal,
-- however the webhook and the return page race.
DROP INDEX credit_entries_one_top_up_per_payment;
CREATE UNIQUE INDEX credit_entries_one_per_payment ON credit_entries (payment_id, kind)
    WHERE payment_id IS NOT NULL;
