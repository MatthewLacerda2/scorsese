-- A user's history, one row per thing that moved their balance. Run scoped:
-- row-level security is the owner filter. See history.rs for the argument.
--
-- $1 project, $2 kind, $3 since (YYYY-MM-DD), $4 until, $5 before (row id),
-- $6 limit. Every filter is optional.
WITH keyed AS (
    -- A reservation and the entries that settle it are one group, keyed by
    -- the reservation; an assistant turn's calls are one group, keyed by its
    -- first (#540); every other entry is a group of its own.
    SELECT id, kind, amount_micros,
           COALESCE(settles,
                    CASE WHEN chat_turn_id IS NOT NULL
                         THEN min(id) OVER (PARTITION BY chat_turn_id) END,
                    id) AS root
    FROM credit_entries
), settled AS (
    SELECT root,
           sum(amount_micros)::bigint AS amount,
           bool_or(kind = 'release') AS released,
           bool_or(kind = 'charge') AS charged
    FROM keyed GROUP BY 1
), rows AS (
    SELECT e.id,
           e.created_at,
           e.project_id,
           pr.name AS project_name,
           CASE WHEN t.id IS NOT NULL THEN 'Assistant: ' || left(t.prompt, 80)
                ELSE e.memo END AS memo,
           CASE WHEN e.veo_generation_id IS NOT NULL THEN 'veo_shot'
                WHEN e.speech_generation_id IS NOT NULL THEN 'spoken_line'
                WHEN e.image_generation_id IS NOT NULL THEN 'still_image'
                WHEN e.voice_design_id IS NOT NULL THEN 'voice_design'
                WHEN e.kind IN ('charge', 'reservation') THEN 'assistant'
                ELSE e.kind END AS kind,
           CASE WHEN e.kind IN ('top_up', 'refund') THEN 'credited'
                WHEN e.kind <> 'reservation' OR s.charged THEN 'charged'
                WHEN s.released THEN 'free'
                ELSE 'pending' END AS status,
           s.amount,
           sum(s.amount) OVER (ORDER BY e.id)::bigint AS balance_after,
           CASE WHEN v.id IS NOT NULL THEN jsonb_build_object(
                    'model', v.model, 'resolution', v.resolution, 'seconds', v.seconds,
                    'aspect', v.aspect, 'prompt', v.prompt,
                    'library_item_id', v.library_item_id, 'error', v.error)
                WHEN p.id IS NOT NULL THEN jsonb_build_object(
                    'model', p.model, 'voice', p.voice, 'text', p.text,
                    'characters', p.characters, 'settings', p.settings,
                    'library_item_id', p.library_item_id, 'error', p.error)
                WHEN i.id IS NOT NULL THEN jsonb_build_object(
                    'model', i.model, 'resolution', i.resolution, 'aspect', i.aspect,
                    'references', i.references_sent, 'prompt', i.prompt,
                    'library_item_id', i.library_item_id, 'error', i.error)
                WHEN d.id IS NOT NULL THEN jsonb_build_object(
                    'prompt', d.prompt, 'passage', d.passage, 'characters', d.characters,
                    'seed', d.seed, 'guidance', d.guidance, 'candidates', d.candidates,
                    'library_item_id', d.library_item_id, 'error', d.error)
                WHEN t.id IS NOT NULL THEN jsonb_build_object(
                    'turn', t.id, 'prompt', t.prompt, 'model', t.model, 'state', t.state,
                    'calls', t.calls, 'input_tokens', t.input_tokens,
                    'output_tokens', t.output_tokens,
                    'cache_write_tokens', t.cache_write_tokens,
                    'cache_read_tokens', t.cache_read_tokens)
                ELSE e.detail END AS detail
    FROM settled s
    JOIN credit_entries e ON e.id = s.root
    LEFT JOIN veo_generations v ON v.id = e.veo_generation_id
    LEFT JOIN speech_generations p ON p.id = e.speech_generation_id
    LEFT JOIN image_generations i ON i.id = e.image_generation_id
    LEFT JOIN voice_designs d ON d.id = e.voice_design_id
    LEFT JOIN chat_turns t ON t.id = e.chat_turn_id
    -- The project's name now; none once it is deleted, the id stays.
    LEFT JOIN projects pr ON pr.id = e.project_id
), matching AS (
    SELECT *,
           count(*) OVER () AS matched,
           (sum(amount) OVER ())::bigint AS total
    FROM rows
    WHERE ($1::bigint IS NULL OR project_id = $1)
      AND ($2::text IS NULL OR kind = $2)
      AND ($3::date IS NULL OR (created_at AT TIME ZONE 'UTC')::date >= $3::date)
      AND ($4::date IS NULL OR (created_at AT TIME ZONE 'UTC')::date <= $4::date)
)
SELECT id,
       extract(epoch FROM created_at)::bigint AS at,
       to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI "UTC"') AS "when",
       kind,
       status,
       project_id,
       project_name,
       memo,
       amount AS amount_micros,
       balance_after AS balance_after_micros,
       detail,
       matched,
       total AS total_micros
FROM matching
WHERE ($5::bigint IS NULL OR id < $5)
ORDER BY id DESC
LIMIT $6
