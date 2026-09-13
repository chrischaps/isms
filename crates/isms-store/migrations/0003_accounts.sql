-- S1.3: identity (TDD 8, 14). Every secret is stored as a hash of a random
-- 256-bit token; the token itself is shown once and never stored.

CREATE TABLE accounts (
    id                  BIGSERIAL PRIMARY KEY,
    email               TEXT NOT NULL UNIQUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    verification        JSONB NOT NULL DEFAULT '{}'::jsonb,
    consent_version     INTEGER NOT NULL DEFAULT 0,
    moderation_state    TEXT NOT NULL DEFAULT 'ok'
);

CREATE TABLE invite_codes (
    code                TEXT PRIMARY KEY,
    created_by          BIGINT REFERENCES accounts (id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    used_by             BIGINT REFERENCES accounts (id),
    used_at             TIMESTAMPTZ
);

CREATE TABLE magic_links (
    token_hash          BYTEA PRIMARY KEY,
    account_id          BIGINT NOT NULL REFERENCES accounts (id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at          TIMESTAMPTZ NOT NULL,
    used_at             TIMESTAMPTZ
);

CREATE TABLE sessions (
    token_hash          BYTEA PRIMARY KEY,
    account_id          BIGINT NOT NULL REFERENCES accounts (id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at          TIMESTAMPTZ NOT NULL,
    last_seen_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX sessions_account ON sessions (account_id);

CREATE TABLE api_keys (
    id                  BIGSERIAL PRIMARY KEY,
    account_id          BIGINT NOT NULL REFERENCES accounts (id),
    society_id          BIGINT NOT NULL REFERENCES societies (id),
    citizen_id          INTEGER NOT NULL,
    label               TEXT NOT NULL,
    prefix              TEXT NOT NULL UNIQUE,
    key_hash            BYTEA NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked_at          TIMESTAMPTZ
);

-- The identity join: one citizen per person per society.
CREATE TABLE citizens (
    society_id          BIGINT NOT NULL REFERENCES societies (id),
    citizen_id          INTEGER NOT NULL,
    account_id          BIGINT NOT NULL REFERENCES accounts (id),
    handle              TEXT NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (society_id, citizen_id),
    UNIQUE (society_id, account_id)
);

GRANT SELECT, INSERT, UPDATE ON accounts, invite_codes, magic_links, sessions, api_keys, citizens TO isms_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON sessions TO isms_app;
GRANT USAGE, SELECT ON SEQUENCE accounts_id_seq, api_keys_id_seq TO isms_app;
