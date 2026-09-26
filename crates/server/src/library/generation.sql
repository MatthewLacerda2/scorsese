-- The generation record behind library item $1, as the item's details carry
-- it: what was asked for, of whom, when, and what it cost. The newest, should
-- two generations have produced the same item. Runs scoped, so only the
-- caller's own records are seen.
SELECT record FROM (
    SELECT g.id, g.created_at, jsonb_build_object(
        'kind', 'veo_shot',
        'id', g.id,
        'model', g.model,
        'prompt', g.prompt,
        'resolution', g.resolution,
        'seconds', g.seconds,
        'aspect', g.aspect,
        'brief_hash', g.brief_hash,
        'project_id', g.project_id,
        'created_at', extract(epoch FROM g.created_at)::bigint,
        'estimated_cost_micros', g.estimated_cost_micros,
        'charged_micros', coalesce((SELECT -sum(e.amount_micros) FROM credit_entries e
                                    WHERE e.veo_generation_id = g.id AND e.kind = 'charge'), 0)
    ) AS record
    FROM veo_generations g WHERE g.library_item_id = $1
    UNION ALL
    SELECT g.id, g.created_at, jsonb_build_object(
        'kind', 'spoken_line',
        'id', g.id,
        'model', g.model,
        'voice', g.voice,
        'text', g.text,
        'settings', g.settings,
        'project_id', g.project_id,
        'created_at', extract(epoch FROM g.created_at)::bigint,
        'estimated_cost_micros', g.estimated_cost_micros,
        'charged_micros', coalesce((SELECT -sum(e.amount_micros) FROM credit_entries e
                                    WHERE e.speech_generation_id = g.id AND e.kind = 'charge'), 0)
    )
    FROM speech_generations g WHERE g.library_item_id = $1
) AS generations
ORDER BY created_at DESC, id DESC
LIMIT 1
