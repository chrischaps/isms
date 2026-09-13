-- S1.1: the event log, snapshots, and the society registry (TDD 8).

CREATE TABLE societies (
    id                  BIGINT PRIMARY KEY,
    name                TEXT NOT NULL UNIQUE,
    preset              TEXT NOT NULL,
    class               TEXT NOT NULL DEFAULT 'canonical',   -- canonical | community
    status              TEXT NOT NULL DEFAULT 'active',      -- active | paused | archived
    seed                BIGINT NOT NULL,
    epoch               INTEGER NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Append-only. seq is assigned by the society actor, never by the database, so
-- the in-memory order and the stored order cannot diverge; a collision on the
-- primary key is the guard against two actors for one society.
CREATE TABLE events (
    society_id          BIGINT NOT NULL REFERENCES societies (id),
    seq                 BIGINT NOT NULL,
    tick                INTEGER NOT NULL,
    cycle               INTEGER NOT NULL,
    epoch               INTEGER NOT NULL,
    kind                TEXT NOT NULL,
    -- serde of isms_core::event::Actor: "system", {"citizen": n}, or null for tick-produced events
    actor               JSONB NOT NULL DEFAULT 'null'::jsonb,
    -- serde of isms_core::kinds::ClientKind ("web", "api_key", ...) or null for tick-produced events
    client_kind         JSONB NOT NULL DEFAULT 'null'::jsonb,
    payload             JSONB NOT NULL,
    received_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (society_id, seq)
);
CREATE INDEX events_society_tick ON events (society_id, tick);
CREATE INDEX events_society_kind ON events (society_id, kind);

-- Belt and braces on top of the role grants below: nobody rewrites history.
CREATE FUNCTION events_append_only() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'events is append-only';
END
$$;
CREATE TRIGGER events_append_only_row
    BEFORE UPDATE OR DELETE ON events
    FOR EACH ROW EXECUTE FUNCTION events_append_only();
CREATE TRIGGER events_append_only_stmt
    BEFORE TRUNCATE ON events
    FOR EACH STATEMENT EXECUTE FUNCTION events_append_only();

-- One row per cycle end: postcard bytes of World, zstd-compressed, with the
-- blake3 hash of the uncompressed bytes and the seq of the last folded event.
CREATE TABLE snapshots (
    society_id          BIGINT NOT NULL REFERENCES societies (id),
    epoch               INTEGER NOT NULL,
    tick                INTEGER NOT NULL,
    last_seq            BIGINT NOT NULL,
    world_hash          BYTEA NOT NULL,
    world               BYTEA NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (society_id, last_seq)
);

-- The application role: INSERT/SELECT on events only. Roles are cluster-wide,
-- so creation is idempotent and race-safe (sqlx::test builds databases in parallel).
DO $$
BEGIN
    BEGIN
        CREATE ROLE isms_app NOLOGIN;
    EXCEPTION WHEN duplicate_object THEN
        NULL;
    END;
END
$$;
GRANT USAGE ON SCHEMA public TO isms_app;
GRANT SELECT, INSERT ON events TO isms_app;
GRANT SELECT, INSERT ON snapshots TO isms_app;
GRANT SELECT, INSERT, UPDATE ON societies TO isms_app;
