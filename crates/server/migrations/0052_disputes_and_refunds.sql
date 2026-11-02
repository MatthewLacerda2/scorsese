-- Disputes and refunds made in Stripe (#1637): a purchase disputed with the
-- card's bank, or refunded from Stripe's Dashboard, takes its credits back.
-- The argument is in crates/server/src/payments/reversed.rs and docs/web.md
-- (Buying credits).
--
-- A refund or a dispute names a charge's payment intent, never the Checkout
-- Session a payment row is keyed on, so the intent is kept when the session
-- is credited. A row credited before this has none, and is found by its
-- session the first time an event names an intent no row has.
ALTER TABLE payments
    ADD COLUMN payment_intent TEXT,
    -- Whether Stripe was asked for the intent of a row credited without
    -- one, so a session that names none is retrieved once, not per event.
    ADD COLUMN intent_asked BOOLEAN NOT NULL DEFAULT false,
    -- How its dispute closed, once Stripe says: a win delivered before the
    -- dispute's opening then leaves the purchase whole.
    ADD COLUMN dispute_closed TEXT,
    -- The most of it Stripe has said is refunded, in centavos: a refund's
    -- event carries everything refunded so far, so this is what the next one
    -- is counted from, and a late or repeated one finds nothing new.
    ADD COLUMN refunded_centavos BIGINT NOT NULL DEFAULT 0
        CHECK (refunded_centavos >= 0 AND refunded_centavos <= brl_centavos);
CREATE UNIQUE INDEX payments_one_per_intent ON payments (payment_intent)
    WHERE payment_intent IS NOT NULL;

-- Three kinds of entry, each linked to its purchase: 'dispute' takes back
-- the whole purchase, past zero if it was spent; 'dispute_won' gives back
-- exactly what that took; 'purchase_refunded' takes back what is left of it
-- in the share refunded.
ALTER TABLE credit_entries
    DROP CONSTRAINT credit_entries_kind,
    DROP CONSTRAINT credit_entries_sign,
    DROP CONSTRAINT credit_entries_payment_needed,
    DROP CONSTRAINT credit_entries_payment_kinds,
    ADD CONSTRAINT credit_entries_kind CHECK (kind IN
        ('top_up', 'reservation', 'release', 'charge', 'refund', 'allowance',
         'adjustment', 'bonus', 'withdrawal', 'dispute', 'dispute_won',
         'purchase_refunded')),
    ADD CONSTRAINT credit_entries_sign CHECK (CASE kind
        WHEN 'top_up'            THEN amount_micros > 0
        WHEN 'release'           THEN amount_micros > 0
        WHEN 'refund'            THEN amount_micros > 0
        WHEN 'bonus'             THEN amount_micros > 0
        WHEN 'dispute_won'       THEN amount_micros > 0
        WHEN 'reservation'       THEN amount_micros < 0
        WHEN 'adjustment'        THEN amount_micros < 0
        WHEN 'withdrawal'        THEN amount_micros < 0
        WHEN 'dispute'           THEN amount_micros < 0
        WHEN 'purchase_refunded' THEN amount_micros < 0
        WHEN 'allowance'         THEN amount_micros <> 0
        ELSE amount_micros <= 0 END),
    ADD CONSTRAINT credit_entries_payment_needed CHECK (kind NOT IN
        ('bonus', 'withdrawal', 'dispute', 'dispute_won', 'purchase_refunded')
        OR payment_id IS NOT NULL),
    ADD CONSTRAINT credit_entries_payment_kinds CHECK (payment_id IS NULL OR kind IN
        ('top_up', 'bonus', 'withdrawal', 'dispute', 'dispute_won', 'purchase_refunded'));

-- One entry of each kind per payment, but for refunds: a second partial
-- refund is an entry of its own.
DROP INDEX credit_entries_one_per_payment;
CREATE UNIQUE INDEX credit_entries_one_per_payment ON credit_entries (payment_id, kind)
    WHERE payment_id IS NOT NULL AND kind <> 'purchase_refunded';

-- One entry per Stripe event at most, however often it is delivered.
CREATE UNIQUE INDEX credit_entries_one_per_stripe_event
    ON credit_entries ((detail ->> 'stripe_event'))
    WHERE detail ? 'stripe_event';
