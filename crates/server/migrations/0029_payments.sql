-- Payments (#548): a user buying credits through Stripe Checkout, in reais.
-- The argument is in crates/server/src/payments/mod.rs and docs/web.md
-- (Buying credits).
--
-- One row per Checkout Session. It is written when the session is created,
-- with the reais asked for, the dollars they will credit and the rate that
-- was locked; it becomes paid once, when Stripe says the money arrived, and
-- that is when its top_up entry is written. Reais are stored here because
-- this is the record of what was charged; the ledger still holds dollars
-- only (#703).
CREATE TABLE payments (
    id               BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id          BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    stripe_session   TEXT NOT NULL UNIQUE,
    brl_centavos     BIGINT NOT NULL CHECK (brl_centavos > 0),
    credit_micros    BIGINT NOT NULL CHECK (credit_micros > 0),
    -- Reais per dollar, exactly as the source sent it, e.g. '5.0096'.
    brl_per_usd      TEXT NOT NULL CHECK (brl_per_usd ~ '^[0-9]+(\.[0-9]+)?$'),
    rate_source      TEXT NOT NULL CHECK (rate_source IN ('awesomeapi', 'bcb_ptax')),
    state            TEXT NOT NULL DEFAULT 'open'
                     CHECK (state IN ('open', 'paid', 'expired')),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    paid_at          TIMESTAMPTZ,
    CHECK ((state = 'paid') = (paid_at IS NOT NULL))
);
CREATE INDEX payments_user_id ON payments (user_id, id DESC);
ALTER TABLE payments ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON payments USING (user_id = (SELECT member_id()));

-- The top-up a payment caused. One per payment, held by the database: the
-- webhook and the return page may both try to credit it, and the second
-- insert is refused here even if everything above it went wrong.
ALTER TABLE credit_entries ADD COLUMN payment_id BIGINT REFERENCES payments (id);
ALTER TABLE credit_entries ADD CHECK (payment_id IS NULL OR kind = 'top_up');
CREATE UNIQUE INDEX credit_entries_one_top_up_per_payment ON credit_entries (payment_id)
    WHERE payment_id IS NOT NULL;
