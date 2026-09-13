-- S1.2: the scheduler's per-society clock (TDD 9.2). Tick n is due at
-- tick_origin + n * tick_seconds; the cycle boundary hour is stored in UTC.
ALTER TABLE societies
    ADD COLUMN tick_seconds        INTEGER NOT NULL DEFAULT 3600,
    ADD COLUMN tick_origin         TIMESTAMPTZ NOT NULL DEFAULT now(),
    ADD COLUMN cycle_boundary_hour INTEGER NOT NULL DEFAULT 4;
