-- The landing page's A/B tests (#1553): who was drawn into which version, and
-- what they did. The rules for writing it are crates/server/src/ab/mod.rs's;
-- docs/web.md (*A/B tests*) has how the operator reads it.
--
-- Not per user, and listed in tests/isolation.rs's NOT_PER_USER with the
-- reason: a visitor is a browser before it is anybody, and most never sign
-- in. Once one does, `user_id` names the account; deleting the account sets
-- it back to NULL and keeps the counts. Only the server writes these,
-- privileged; no member may read them.

-- One browser, by the random code its `scorsese_visitor` cookie carries.
CREATE TABLE ab_visitors (
    code       UUID PRIMARY KEY,
    first_seen TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Where Cloudflare placed the first visit, from the tunnel's door only;
    -- NULL elsewhere. ISO 3166 country, and its subdivision's code.
    country    TEXT CHECK (country ~ '^[A-Z0-9]{2}$'),
    region     TEXT CHECK (region ~ '^[A-Z0-9]{1,3}$'),
    user_id    BIGINT REFERENCES users (id) ON DELETE SET NULL,
    linked_at  TIMESTAMPTZ,
    CHECK ((user_id IS NULL) OR (linked_at IS NOT NULL))
);
CREATE INDEX ab_visitors_user ON ab_visitors (user_id) WHERE user_id IS NOT NULL;

-- The version of each test a visitor was first shown.
CREATE TABLE ab_exposures (
    visitor UUID NOT NULL REFERENCES ab_visitors (code) ON DELETE CASCADE,
    test    TEXT NOT NULL CHECK (test ~ '^[a-z0-9-]{1,32}$'),
    version TEXT NOT NULL CHECK (version ~ '^[a-z0-9-]{1,32}$'),
    at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (visitor, test)
);
CREATE INDEX ab_exposures_test ON ab_exposures (test, version);

-- What a visitor did: 'view' (the landing was shown), 'section' (value: the
-- deepest section seen, counted from 1), 'end' (the last section was seen),
-- 'time' (value: seconds the page was visible, at most 30 minutes),
-- 'prompt' (the prompt box was sent).
CREATE TABLE ab_events (
    id      BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    visitor UUID NOT NULL REFERENCES ab_visitors (code) ON DELETE CASCADE,
    kind    TEXT NOT NULL CHECK (kind IN ('view', 'section', 'end', 'time', 'prompt')),
    value   INTEGER NOT NULL DEFAULT 0 CHECK (CASE kind
        WHEN 'section' THEN value BETWEEN 1 AND 64
        WHEN 'time'    THEN value BETWEEN 0 AND 1800
        ELSE value = 0 END),
    at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ab_events_visitor_at ON ab_events (visitor, at);

REVOKE ALL ON ab_visitors, ab_exposures, ab_events FROM scorsese_member;
