-- Adjustments (#1542): the operator taking money back out of a balance,
-- with the reason, as one new negative entry. The ledger stays append-only:
-- a reset to zero is an entry that sums the balance to zero, never an edit.
-- The argument is in crates/server/src/credits/ledger.rs (adjust).
ALTER TABLE credit_entries
    DROP CONSTRAINT credit_entries_kind,
    DROP CONSTRAINT credit_entries_sign,
    ADD CONSTRAINT credit_entries_kind CHECK (kind IN
        ('top_up', 'reservation', 'release', 'charge', 'refund', 'allowance', 'adjustment')),
    ADD CONSTRAINT credit_entries_sign CHECK (CASE kind
        WHEN 'top_up'      THEN amount_micros > 0
        WHEN 'release'     THEN amount_micros > 0
        WHEN 'refund'      THEN amount_micros > 0
        WHEN 'reservation' THEN amount_micros < 0
        WHEN 'adjustment'  THEN amount_micros < 0
        WHEN 'allowance'   THEN amount_micros <> 0
        ELSE amount_micros <= 0 END);
