-- What is left of a user's free allowance at an instant, and when it renews.
-- Run scoped: row-level security is the owner filter. See allowance.rs.
--
-- $1 the instant, in seconds since 1970; NULL is now.
WITH here AS (
    SELECT COALESCE(to_timestamp($1::bigint), now()) AT TIME ZONE 'America/Sao_Paulo' AS local
), bounds AS (
    SELECT date_trunc('day', local) AS day,
           date_trunc('day', local) + interval '1 day' AS tomorrow,
           date_trunc('month', local) AS month,
           date_trunc('month', local) + interval '1 month' AS next_month
    FROM here
), granted AS (
    -- Every grant of the month that was not given back, in Brasília time.
    SELECT g.amount_micros, g.created_at AT TIME ZONE 'America/Sao_Paulo' AS local
    FROM credit_entries g, bounds b
    WHERE g.kind = 'allowance' AND g.amount_micros > 0
      AND g.created_at >= b.month AT TIME ZONE 'America/Sao_Paulo'
      AND g.created_at < b.next_month AT TIME ZONE 'America/Sao_Paulo'
      AND NOT EXISTS (SELECT 1 FROM credit_entries r
                      WHERE r.kind = 'allowance' AND r.amount_micros < 0
                        AND r.settles = g.settles)
), used AS (
    SELECT COALESCE(sum(g.amount_micros)
                    FILTER (WHERE g.local >= b.day AND g.local < b.tomorrow), 0)::bigint AS day,
           COALESCE(sum(g.amount_micros), 0)::bigint AS month
    FROM bounds b LEFT JOIN granted g ON true
)
SELECT greatest(0, least(a.daily_micros - u.day, a.monthly_micros - u.month))::bigint
           AS left_micros,
       -- Nothing renews when nothing is given away.
       CASE WHEN a.daily_micros = 0 OR a.monthly_micros = 0 THEN 0
            ELSE extract(epoch FROM
                CASE WHEN a.monthly_micros - u.month <= 0 THEN b.next_month ELSE b.tomorrow END
                AT TIME ZONE 'America/Sao_Paulo')::bigint END AS renews_at,
       u.day AS used_today_micros,
       u.month AS used_this_month_micros
FROM free_allowance a, used u, bounds b
