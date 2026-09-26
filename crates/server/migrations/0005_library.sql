-- A user's library (#535): every file they uploaded or generated, reusable in
-- any of their projects, and the uploads still on their way in. The argument
-- is in crates/server/src/library/mod.rs and docs/web.md (Library).

-- One file. Stored once per (user, sha256): the bytes sit on disk at
-- users/<user id>/library/<sha256>.<extension> under the storage root, so the
-- hash and the extension are the whole of where it is -- no path column that
-- could disagree with them.
--
-- `kind` is what the file is (the three kinds a file can be imported as),
-- never whether it was generated: a generated item is the one with a
-- `brief_hash`, the hash of what it was generated from, which is how a second
-- request for the same brief in another of the same user's projects finds it
-- instead of paying again. Unique per user, never across users (#527).
CREATE TABLE library_items (
    id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id      BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    sha256       TEXT NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    name         TEXT NOT NULL CHECK (btrim(name) <> ''),
    kind         TEXT NOT NULL CHECK (kind IN ('video', 'image', 'audio')),
    extension    TEXT NOT NULL CHECK (extension ~ '^[a-z0-9]{1,8}$'),
    size_bytes   BIGINT NOT NULL CHECK (size_bytes >= 0),
    -- scorsese_core::MediaMetadata, exactly as a project's assets table
    -- records it, so placing the item in a project copies it unchanged.
    media        JSONB NOT NULL CHECK (jsonb_typeof(media) = 'object'),
    -- Words for the assistant to read when choosing a file; optional.
    description  TEXT,
    brief_hash   TEXT CHECK (brief_hash ~ '^[0-9a-f]{64}$'),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at TIMESTAMPTZ,
    UNIQUE (user_id, sha256),
    UNIQUE (user_id, brief_hash),
    -- For the generation records' key below: what stops one pointing at
    -- another user's item.
    UNIQUE (id, user_id)
);
CREATE INDEX library_items_newest ON library_items (user_id, id DESC);
ALTER TABLE library_items ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON library_items USING (user_id = (SELECT member_id()));

-- A resumable upload in progress (tus). The bytes so far are a file in the
-- server's rebuildable cache, and how many there are is that file's length:
-- the row holds only what the upload was announced as. `sha256` is the
-- browser's claim, checked against the bytes before anything reaches the
-- library.
CREATE TABLE uploads (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name       TEXT NOT NULL CHECK (btrim(name) <> ''),
    kind       TEXT NOT NULL CHECK (kind IN ('video', 'image', 'audio')),
    extension  TEXT NOT NULL CHECK (extension ~ '^[a-z0-9]{1,8}$'),
    sha256     TEXT NOT NULL CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    length     BIGINT NOT NULL CHECK (length >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    touched_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX uploads_user_id ON uploads (user_id);
ALTER TABLE uploads ENABLE ROW LEVEL SECURITY;
CREATE POLICY own_rows ON uploads USING (user_id = (SELECT member_id()));

-- Every file a stored project names is one of its owner's library items.
-- 0003 left this key to the library; with it, a document cannot name a file
-- its owner does not have, and a file a project uses cannot be deleted (NO
-- ACTION, checked at the end of the statement, so deleting an account --
-- which cascades to both tables -- still works).
--
-- Rows written before the library existed name files nobody could have
-- uploaded. There should be none; if there are, the start stops here and says
-- which, rather than the key failing with a bare constraint name.
DO $$
DECLARE
    orphan record;
BEGIN
    SELECT project_id, sha256 INTO orphan FROM project_assets LIMIT 1;
    IF FOUND THEN
        RAISE EXCEPTION 'project % names file % and no library holds it', orphan.project_id, orphan.sha256
            USING HINT = 'nothing could upload a file before this migration, so no such file exists: '
                         'remove that asset from projects.document with psql, then start again';
    END IF;
END $$;

ALTER TABLE project_assets
    ADD CONSTRAINT project_assets_library_item
    FOREIGN KEY (user_id, sha256) REFERENCES library_items (user_id, sha256);

-- A generation record names the item it produced (0004 left the key to the
-- library). By (item, owner), so a record cannot name another user's item.
-- Deleting the item keeps the record -- it is the account of money spent --
-- and forgets only which item it was.
ALTER TABLE veo_generations
    ADD CONSTRAINT veo_generations_library_item
    FOREIGN KEY (library_item_id, user_id) REFERENCES library_items (id, user_id)
    ON DELETE SET NULL (library_item_id);
ALTER TABLE speech_generations
    ADD CONSTRAINT speech_generations_library_item
    FOREIGN KEY (library_item_id, user_id) REFERENCES library_items (id, user_id)
    ON DELETE SET NULL (library_item_id);
CREATE INDEX veo_generations_library_item ON veo_generations (library_item_id);
CREATE INDEX speech_generations_library_item ON speech_generations (library_item_id);
