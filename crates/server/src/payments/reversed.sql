-- A paid payment by its payment intent, locked, with what was taken of it
-- already. Run scoped as its owner, after the owner's lock (reversed.rs).
--
-- $1 the payment intent.
SELECT p.id,
       p.stripe_session,
       p.brl_centavos,
       p.credit_micros,
       p.bonus_micros,
       p.dispute_closed,
       p.refunded_centavos,
       to_char(p.paid_at AT TIME ZONE 'America/Sao_Paulo', 'FMDD Mon') AS paid_on,
       (SELECT COALESCE(-sum(e.amount_micros), 0)::bigint FROM credit_entries e
        WHERE e.payment_id = p.id
          AND e.kind IN ('withdrawal', 'dispute', 'dispute_won', 'purchase_refunded'))
           AS taken,
       (SELECT -e.amount_micros FROM credit_entries e
        WHERE e.payment_id = p.id AND e.kind = 'dispute') AS disputed,
       (SELECT (e.detail ->> 'refunded_before')::bigint FROM credit_entries e
        WHERE e.payment_id = p.id AND e.kind = 'dispute') AS refunded_before,
       EXISTS (SELECT 1 FROM credit_entries e
               WHERE e.payment_id = p.id AND e.kind = 'dispute_won') AS won,
       (SELECT COALESCE(sum((e.detail ->> 'refund_centavos')::bigint), 0)::bigint
        FROM credit_entries e
        WHERE e.payment_id = p.id AND e.kind = 'withdrawal') AS withdrawn_centavos
FROM payments p
WHERE p.payment_intent = $1 AND p.state = 'paid'
FOR UPDATE OF p
