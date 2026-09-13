//! Identity tables (TDD 8, 14): accounts, invite codes, magic links,
//! sessions, API keys, and the citizen join. Secrets arrive here already
//! hashed; this module never sees a token.

use crate::{PgEventStore, Result, StoreError};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountRow {
    pub id: i64,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub consent_version: i32,
    pub moderation_state: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiKeyRow {
    pub id: i64,
    pub account_id: i64,
    pub society_id: i64,
    pub citizen_id: i32,
    pub label: String,
    pub prefix: String,
    pub key_hash: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CitizenRow {
    pub society_id: i64,
    pub citizen_id: i32,
    pub account_id: i64,
    pub handle: String,
}

impl PgEventStore {
    // -- invites and accounts -------------------------------------------------

    pub async fn create_invite_code(&self, code: &str, created_by: Option<i64>) -> Result<()> {
        sqlx::query!(
            "INSERT INTO invite_codes (code, created_by) VALUES ($1, $2)",
            code,
            created_by
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn unused_invite_codes(&self) -> Result<Vec<String>> {
        let rows =
            sqlx::query!("SELECT code FROM invite_codes WHERE used_by IS NULL ORDER BY created_at")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|r| r.code).collect())
    }

    pub async fn account_by_email(&self, email: &str) -> Result<Option<AccountRow>> {
        let row = sqlx::query_as!(
            AccountRow,
            "SELECT id, email, created_at, consent_version, moderation_state FROM accounts WHERE email = $1",
            email
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn account_by_id(&self, id: i64) -> Result<Option<AccountRow>> {
        let row = sqlx::query_as!(
            AccountRow,
            "SELECT id, email, created_at, consent_version, moderation_state FROM accounts WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Redeem an unused invite code and create the account in one transaction.
    /// `Ok(None)` when the code is unknown or already used.
    pub async fn create_account_with_invite(
        &self,
        email: &str,
        code: &str,
    ) -> Result<Option<AccountRow>> {
        let mut tx = self.pool.begin().await?;
        let claimed = sqlx::query!(
            "SELECT code FROM invite_codes WHERE code = $1 AND used_by IS NULL FOR UPDATE",
            code
        )
        .fetch_optional(&mut *tx)
        .await?;
        if claimed.is_none() {
            tx.rollback().await?;
            return Ok(None);
        }
        let account = sqlx::query_as!(
            AccountRow,
            "INSERT INTO accounts (email) VALUES ($1)
             RETURNING id, email, created_at, consent_version, moderation_state",
            email
        )
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query!(
            "UPDATE invite_codes SET used_by = $1, used_at = now() WHERE code = $2",
            account.id,
            code
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(account))
    }

    /// Create the account if it does not exist (admin and e2e use; no invite).
    pub async fn ensure_account(&self, email: &str) -> Result<AccountRow> {
        if let Some(a) = self.account_by_email(email).await? {
            return Ok(a);
        }
        let row = sqlx::query_as!(
            AccountRow,
            "INSERT INTO accounts (email) VALUES ($1)
             ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
             RETURNING id, email, created_at, consent_version, moderation_state",
            email
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    // -- magic links and sessions ---------------------------------------------

    pub async fn create_magic_link(
        &self,
        token_hash: &[u8],
        account_id: i64,
        expires_at: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO magic_links (token_hash, account_id, expires_at) VALUES ($1, $2, $3)",
            token_hash,
            account_id,
            expires_at
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Mark a link used and return its account; `None` if unknown, used, or expired.
    pub async fn consume_magic_link(&self, token_hash: &[u8]) -> Result<Option<i64>> {
        let row = sqlx::query!(
            "UPDATE magic_links SET used_at = now()
             WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()
             RETURNING account_id",
            token_hash
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.account_id))
    }

    pub async fn create_session(
        &self,
        token_hash: &[u8],
        account_id: i64,
        expires_at: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO sessions (token_hash, account_id, expires_at) VALUES ($1, $2, $3)",
            token_hash,
            account_id,
            expires_at
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// The account behind a live session, touching `last_seen_at`.
    pub async fn session_account(&self, token_hash: &[u8]) -> Result<Option<i64>> {
        let row = sqlx::query!(
            "UPDATE sessions SET last_seen_at = now()
             WHERE token_hash = $1 AND expires_at > now() RETURNING account_id",
            token_hash
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.account_id))
    }

    pub async fn delete_session(&self, token_hash: &[u8]) -> Result<()> {
        sqlx::query!("DELETE FROM sessions WHERE token_hash = $1", token_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // -- api keys ------------------------------------------------------------

    pub async fn create_api_key(
        &self,
        account_id: i64,
        society_id: i64,
        citizen_id: i32,
        label: &str,
        prefix: &str,
        key_hash: &[u8],
    ) -> Result<ApiKeyRow> {
        let row = sqlx::query_as!(
            ApiKeyRow,
            "INSERT INTO api_keys (account_id, society_id, citizen_id, label, prefix, key_hash)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, account_id, society_id, citizen_id, label, prefix, key_hash, created_at, revoked_at",
            account_id,
            society_id,
            citizen_id,
            label,
            prefix,
            key_hash
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn api_key_by_prefix(&self, prefix: &str) -> Result<Option<ApiKeyRow>> {
        let row = sqlx::query_as!(
            ApiKeyRow,
            "SELECT id, account_id, society_id, citizen_id, label, prefix, key_hash, created_at, revoked_at
             FROM api_keys WHERE prefix = $1",
            prefix
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_api_keys(&self, account_id: i64) -> Result<Vec<ApiKeyRow>> {
        let rows = sqlx::query_as!(
            ApiKeyRow,
            "SELECT id, account_id, society_id, citizen_id, label, prefix, key_hash, created_at, revoked_at
             FROM api_keys WHERE account_id = $1 AND revoked_at IS NULL ORDER BY id",
            account_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Revoke a key the account owns; `false` when there is no such live key.
    pub async fn revoke_api_key(&self, id: i64, account_id: i64) -> Result<bool> {
        let done = sqlx::query!(
            "UPDATE api_keys SET revoked_at = now()
             WHERE id = $1 AND account_id = $2 AND revoked_at IS NULL",
            id,
            account_id
        )
        .execute(&self.pool)
        .await?;
        Ok(done.rows_affected() == 1)
    }

    // -- citizens ------------------------------------------------------------

    pub async fn citizen_of(&self, society_id: i64, account_id: i64) -> Result<Option<CitizenRow>> {
        let row = sqlx::query_as!(
            CitizenRow,
            "SELECT society_id, citizen_id, account_id, handle FROM citizens
             WHERE society_id = $1 AND account_id = $2",
            society_id,
            account_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn citizenships(&self, account_id: i64) -> Result<Vec<CitizenRow>> {
        let rows = sqlx::query_as!(
            CitizenRow,
            "SELECT society_id, citizen_id, account_id, handle FROM citizens
             WHERE account_id = $1 ORDER BY society_id",
            account_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// `Ok(false)` when the account already has a citizen in this society.
    pub async fn insert_citizen(&self, row: &CitizenRow) -> Result<bool> {
        let result = sqlx::query!(
            "INSERT INTO citizens (society_id, citizen_id, account_id, handle) VALUES ($1, $2, $3, $4)",
            row.society_id,
            row.citizen_id,
            row.account_id,
            row.handle
        )
        .execute(&self.pool)
        .await;
        match result {
            Ok(_) => Ok(true),
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => Ok(false),
            Err(e) => Err(StoreError::from(e)),
        }
    }
}
