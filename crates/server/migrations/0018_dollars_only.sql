-- Dollars only (#703): the ledger tracks dollars and nothing else. A converted
-- figure is wrong the day after it is shown, so money is never shown in
-- another currency, and nothing about one is stored.
--
-- display_rates was the operator's reais-per-dollar rate balances were shown
-- at; it goes. A manual top-up recorded the reais that arrived and the rate
-- they were converted at in its detail; a top-up now records the dollars
-- credited and nothing else, so those keys go from the rows that carry them.
-- The stored values were test data, and nothing is carried over.
DROP TABLE display_rates;

-- credit_entries is append-only, and its trigger refuses an UPDATE from
-- everybody (0004). This one rewrite of history is the schema's own, made
-- once, in this transaction: the trigger is lifted around it and restored.
ALTER TABLE credit_entries DISABLE TRIGGER append_only;
UPDATE credit_entries
SET detail = detail - 'reais_centavos' - 'brl_per_usd_e4'
WHERE kind = 'top_up' AND (detail ? 'reais_centavos' OR detail ? 'brl_per_usd_e4');
ALTER TABLE credit_entries ENABLE TRIGGER append_only;
