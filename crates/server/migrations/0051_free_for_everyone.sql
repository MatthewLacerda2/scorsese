-- A ceiling on free spend for everyone together (#1658): all accounts'
-- allowances draw on one operator-wide pot each Brasília day, so the number
-- of accounts cannot multiply what the operator gives away. The argument is
-- in crates/server/src/credits/allowance.rs.

-- The pot, as the server was last started with (SCORSESE_FREE_TOTAL_DAILY).
-- Nothing until a server says otherwise, like the rest of the row.
ALTER TABLE free_allowance
    ADD COLUMN total_daily_micros BIGINT NOT NULL DEFAULT 0
        CHECK (total_daily_micros >= 0);

-- What every account together was granted free in the Brasília day holding
-- `at`, less what failures gave back. Cross-user by nature, so it runs as
-- the owner of credit_entries, past row-level security, and answers one sum:
-- no row, no user. A member may call it and nothing else of the kind.
CREATE FUNCTION free_granted_by_everyone(at timestamptz) RETURNS bigint
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, public AS $$
    WITH local AS (
        SELECT date_trunc('day', at AT TIME ZONE 'America/Sao_Paulo') AS day
    ), bounds AS (
        SELECT day AT TIME ZONE 'America/Sao_Paulo' AS starts,
               (day + interval '1 day') AT TIME ZONE 'America/Sao_Paulo' AS ends
        FROM local
    )
    SELECT COALESCE(sum(g.amount_micros), 0)::bigint
    FROM credit_entries g, bounds b
    WHERE g.kind = 'allowance' AND g.amount_micros > 0
      AND g.created_at >= b.starts AND g.created_at < b.ends
      AND NOT EXISTS (SELECT 1 FROM credit_entries r
                      WHERE r.kind = 'allowance' AND r.amount_micros < 0
                        AND r.settles = g.settles)
$$;
REVOKE EXECUTE ON FUNCTION free_granted_by_everyone(timestamptz) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION free_granted_by_everyone(timestamptz) TO scorsese_member;

-- The day's grants, found without reading every entry.
CREATE INDEX credit_entries_grants_by_time ON credit_entries (created_at)
    WHERE kind = 'allowance' AND amount_micros > 0;
