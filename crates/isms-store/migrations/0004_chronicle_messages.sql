-- S1.5: the Chronicle projection (regenerable from events) and free-text
-- messages (D12: stored outside the event log, same retention).

CREATE TABLE chronicle (
    society_id          BIGINT NOT NULL REFERENCES societies (id),
    source_event_seq    BIGINT NOT NULL,
    ordinal             INTEGER NOT NULL DEFAULT 0,
    cycle               INTEGER NOT NULL,
    tick                INTEGER NOT NULL,
    headline            TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (society_id, source_event_seq, ordinal)
);
CREATE INDEX chronicle_society_cycle ON chronicle (society_id, cycle, source_event_seq);

CREATE TABLE messages (
    id                  BIGSERIAL PRIMARY KEY,
    society_id          BIGINT NOT NULL REFERENCES societies (id),
    -- square | org:<org id> | dm:<lower citizen id>:<higher citizen id>
    channel             TEXT NOT NULL,
    sender_citizen      INTEGER NOT NULL,
    body                TEXT NOT NULL,
    tick                INTEGER NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX messages_society_channel ON messages (society_id, channel, id);

GRANT SELECT, INSERT, DELETE ON chronicle TO isms_app;
GRANT SELECT, INSERT ON messages TO isms_app;
GRANT USAGE, SELECT ON SEQUENCE messages_id_seq TO isms_app;
