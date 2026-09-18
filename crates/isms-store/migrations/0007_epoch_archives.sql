-- S1.15 (GDD 11.5, TDD 12): what an epoch leaves behind. One row per ended
-- epoch, written in the same transaction as its EpochEnded event so a crash
-- cannot leave one without the other. `summary` is the engine's frozen
-- EpochSummary (the Observatory snapshot Phase 3 reads); `closing_statements`
-- is the citizens' word, one entry each, editable until `closes_at`. Neither
-- is an economic fact: nothing here replays.
CREATE TABLE epoch_archives (
    society_id          BIGINT NOT NULL REFERENCES societies (id),
    epoch               INTEGER NOT NULL,
    ended_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- scheduled | collapse | operator
    reason              TEXT NOT NULL,
    final_cycle         INTEGER NOT NULL,
    ended_seq           BIGINT NOT NULL,
    summary             JSONB NOT NULL,
    closes_at           TIMESTAMPTZ NOT NULL,
    -- [{ citizen, handle, text, written_at }], in arrival order
    closing_statements  JSONB NOT NULL DEFAULT '[]'::jsonb,
    PRIMARY KEY (society_id, epoch)
);

GRANT SELECT, INSERT, UPDATE ON epoch_archives TO isms_app;
