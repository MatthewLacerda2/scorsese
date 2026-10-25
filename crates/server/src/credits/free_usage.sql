-- What the free allowance paid (#1530), per user per Brasília day, with the
-- month to date and the day each user first paid. Every user's: run
-- privileged, by the operator (`scorsese-server credit free-usage`).
--
-- $1 email, $2 since (YYYY-MM-DD), $3 until. Every filter is optional; the
-- month to date counts the whole month whatever the range shows.
WITH granted AS (
    -- Every grant not given back by a failure.
    SELECT g.user_id, (g.created_at AT TIME ZONE 'America/Sao_Paulo')::date AS day,
           g.amount_micros
    FROM credit_entries g
    WHERE g.kind = 'allowance' AND g.amount_micros > 0
      AND NOT EXISTS (SELECT 1 FROM credit_entries r
                      WHERE r.kind = 'allowance' AND r.amount_micros < 0
                        AND r.settles = g.settles)
), days AS (
    SELECT user_id, day, sum(amount_micros)::bigint AS free
    FROM granted GROUP BY 1, 2
), counted AS (
    SELECT user_id, day, free,
           sum(free) OVER (PARTITION BY user_id, date_trunc('month', day) ORDER BY day)::bigint
               AS month_to_date
    FROM days
)
SELECT u.email,
       c.day::text AS day,
       c.free,
       c.month_to_date,
       (SELECT min(t.created_at AT TIME ZONE 'America/Sao_Paulo')::date::text
        FROM credit_entries t WHERE t.user_id = c.user_id AND t.kind = 'top_up') AS first_paid
FROM counted c JOIN users u ON u.id = c.user_id
WHERE ($1::text IS NULL OR u.email = $1)
  AND ($2::date IS NULL OR c.day >= $2::date)
  AND ($3::date IS NULL OR c.day <= $3::date)
ORDER BY u.email, c.day
