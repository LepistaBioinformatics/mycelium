use crate::{
    config::SqliteDbPoolProvider, models::token::Token as TokenModel,
    schema::token, types::naive_timestamp_to_text,
};
use diesel::prelude::*;

use async_trait::async_trait;
use chrono::{DateTime, Local};
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        token::{
            EmailConfirmationTokenMeta, MagicLinkTokenMeta, MultiTypeMeta,
            PasswordChangeTokenMeta, Token, UserAccountConnectionString,
        },
    },
    entities::TokenRegistration,
};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = TokenRegistration)]
pub struct TokenRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl TokenRegistration for TokenRegistrationSqlDbRepository {
    #[tracing::instrument(name = "create_email_confirmation_token", skip_all)]
    async fn create_email_confirmation_token(
        &self,
        meta: EmailConfirmationTokenMeta,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors> {
        let mut meta_clone = meta.clone();
        if let Err(err) = meta_clone.encrypted_token() {
            return creation_err(format!(
                "Unexpected error detected on token processing: {}",
                err
            ))
            .as_error();
        }

        let token = self.insert(meta_clone, expires)?;
        let meta: EmailConfirmationTokenMeta =
            serde_json::from_str(&token.meta).unwrap();

        Ok(CreateResponseKind::Created(Token::new(
            Some(token.id),
            self.expiration_to_local(&token.expiration),
            MultiTypeMeta::EmailConfirmation(meta),
        )))
    }

    #[tracing::instrument(name = "create_password_change_token", skip_all)]
    async fn create_password_change_token(
        &self,
        meta: PasswordChangeTokenMeta,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors> {
        let mut meta_clone = meta.clone();
        if let Err(err) = meta_clone.encrypted_token() {
            return creation_err(format!(
                "Unexpected error detected on token processing: {}",
                err
            ))
            .as_error();
        }

        let token = self.insert(meta_clone, expires)?;
        let meta: PasswordChangeTokenMeta =
            serde_json::from_str(&token.meta).unwrap();

        Ok(CreateResponseKind::Created(Token::new(
            Some(token.id),
            self.expiration_to_local(&token.expiration),
            MultiTypeMeta::PasswordChange(meta),
        )))
    }

    #[tracing::instrument(name = "create_connection_string", skip_all)]
    async fn create_connection_string(
        &self,
        meta: UserAccountConnectionString,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors> {
        let mut meta_clone = meta.clone();
        if let Err(err) = meta_clone.encrypted_token() {
            return creation_err(format!(
                "Unexpected error detected on token processing: {}",
                err
            ))
            .as_error();
        }

        let token = self.insert(meta_clone, expires)?;
        let meta: UserAccountConnectionString =
            serde_json::from_str(&token.meta).unwrap();

        Ok(CreateResponseKind::Created(Token::new(
            Some(token.id),
            self.expiration_to_local(&token.expiration),
            MultiTypeMeta::UserAccountConnectionString(meta),
        )))
    }

    #[tracing::instrument(name = "create_magic_link_token", skip_all)]
    async fn create_magic_link_token(
        &self,
        meta: MagicLinkTokenMeta,
        expires: DateTime<Local>,
    ) -> Result<CreateResponseKind<Token>, MappedErrors> {
        // No encryption needed — token is a UUID, code is not secret
        // once displayed; the DB is the source of truth for validity.
        let token = self.insert(meta, expires)?;
        let meta: MagicLinkTokenMeta =
            serde_json::from_str(&token.meta).unwrap();

        Ok(CreateResponseKind::Created(Token::new(
            Some(token.id),
            self.expiration_to_local(&token.expiration),
            MultiTypeMeta::MagicLink(meta),
        )))
    }
}

impl TokenRegistrationSqlDbRepository {
    fn insert<M: serde::Serialize>(
        &self,
        meta: M,
        expires: DateTime<Local>,
    ) -> Result<TokenModel, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let meta_text = serde_json::to_string(&meta)
            .map_err(|_| creation_err("Could not serialize the meta data"))?;

        diesel::insert_into(token::table)
            .values((
                token::meta.eq(meta_text),
                token::expiration
                    .eq(naive_timestamp_to_text(&expires.naive_utc())),
            ))
            .returning(TokenModel::as_returning())
            .get_result::<TokenModel>(conn)
            .map_err(|e| {
                creation_err(format!(
                    "Unexpected error detected on create record: {}",
                    e
                ))
            })
    }

    fn expiration_to_local(&self, expiration: &str) -> DateTime<Local> {
        crate::types::naive_timestamp_from_text(expiration)
            .unwrap()
            .and_local_timezone(Local)
            .unwrap()
    }
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        repositories::token::TokenInvalidationSqlDbRepository,
        test_support::setup_temp_db,
    };
    use chrono::Duration;
    use myc_core::domain::{
        dtos::{
            email::Email,
            token::{MagicLinkTokenMeta, MAX_FAILED_MAGIC_LINK_ATTEMPTS},
        },
        entities::TokenInvalidation,
    };
    use uuid::Uuid;

    fn email_confirmation_meta(
        user_id: Uuid,
        email: &Email,
        raw_token: &str,
    ) -> EmailConfirmationTokenMeta {
        serde_json::from_value(serde_json::json!({
            "userId": user_id,
            "email": { "username": email.username, "domain": email.domain },
            "token": raw_token,
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn email_confirmation_token_round_trips_through_sqlite(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let registration = TokenRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let invalidation = TokenInvalidationSqlDbRepository {
            db_config: db.provider.clone(),
        };

        let user_id = Uuid::new_v4();
        let email = Email::from_string("owner@acme.test".into())?;
        let raw_token = "raw-confirmation-token";
        let meta = email_confirmation_meta(user_id, &email, raw_token);

        registration
            .create_email_confirmation_token(
                meta.clone(),
                Local::now() + Duration::hours(1),
            )
            .await?;

        // Wrong token must not match
        let wrong_lookup =
            email_confirmation_meta(user_id, &email, "wrong-token-value");
        let not_found = invalidation
            .get_and_invalidate_email_confirmation_token(wrong_lookup)
            .await?;
        assert!(matches!(
            not_found,
            mycelium_base::entities::FetchResponseKind::NotFound(_)
        ));

        // Correct token invalidates and returns the user id
        let lookup = email_confirmation_meta(user_id, &email, raw_token);
        let found = invalidation
            .get_and_invalidate_email_confirmation_token(lookup.clone())
            .await?;
        match found {
            mycelium_base::entities::FetchResponseKind::Found(id) => {
                assert_eq!(id, user_id)
            }
            mycelium_base::entities::FetchResponseKind::NotFound(_) => {
                panic!("expected the token to be found and invalidated")
            }
        }

        // Token is single-use: the same lookup must now fail
        let consumed = invalidation
            .get_and_invalidate_email_confirmation_token(lookup)
            .await?;
        assert!(matches!(
            consumed,
            mycelium_base::entities::FetchResponseKind::NotFound(_)
        ));

        Ok(())
    }

    #[tokio::test]
    async fn magic_link_two_phase_consumption_round_trips_through_sqlite(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let registration = TokenRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let invalidation = TokenInvalidationSqlDbRepository {
            db_config: db.provider.clone(),
        };

        let email = Email::from_string("owner@acme.test".into())?;
        let meta = MagicLinkTokenMeta::new(email.clone());
        let display_token = meta.token.clone().expect("token must be set");

        registration
            .create_magic_link_token(
                meta.clone(),
                Local::now() + Duration::minutes(15),
            )
            .await?;

        // Phase 1: open the display link, consume the token, get the code
        let code = match invalidation
            .get_code_and_invalidate_display_token(&email, &display_token)
            .await?
        {
            mycelium_base::entities::FetchResponseKind::Found(code) => code,
            mycelium_base::entities::FetchResponseKind::NotFound(_) => {
                panic!("expected the display token to be found")
            }
        };
        assert_eq!(code, meta.code);

        // Re-opening the same display link must now fail (token consumed)
        let reopened = invalidation
            .get_code_and_invalidate_display_token(&email, &display_token)
            .await?;
        assert!(matches!(
            reopened,
            mycelium_base::entities::FetchResponseKind::NotFound(_)
        ));

        // Phase 2: submit the code, record is deleted
        let verified = invalidation
            .get_and_invalidate_magic_link_code(&email, &code)
            .await?;
        assert!(matches!(
            verified,
            mycelium_base::entities::FetchResponseKind::Found(())
        ));

        // Code is single-use: resubmitting must fail
        let reused = invalidation
            .get_and_invalidate_magic_link_code(&email, &code)
            .await?;
        assert!(matches!(
            reused,
            mycelium_base::entities::FetchResponseKind::NotFound(_)
        ));

        Ok(())
    }
    async fn open_magic_link(
        registration: &TokenRegistrationSqlDbRepository,
        invalidation: &TokenInvalidationSqlDbRepository,
        email: &Email,
    ) -> Result<String, MappedErrors> {
        let meta = MagicLinkTokenMeta::new(email.clone());
        let display_token = meta.token.clone().expect("token must be set");

        registration
            .create_magic_link_token(meta, Local::now() + Duration::minutes(15))
            .await?;

        match invalidation
            .get_code_and_invalidate_display_token(email, &display_token)
            .await?
        {
            mycelium_base::entities::FetchResponseKind::Found(code) => Ok(code),
            mycelium_base::entities::FetchResponseKind::NotFound(_) => {
                panic!("expected the display token to be found")
            }
        }
    }

    fn wrong_code(code: &str) -> String {
        format!("{:06}", (code.parse::<u32>().unwrap() + 1) % 1_000_000)
    }

    #[tokio::test]
    async fn magic_link_code_is_discarded_after_max_failed_attempts(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let registration = TokenRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let invalidation = TokenInvalidationSqlDbRepository {
            db_config: db.provider.clone(),
        };

        let email = Email::from_string("owner@acme.test".into())?;
        let code =
            open_magic_link(&registration, &invalidation, &email).await?;

        for _ in 0..MAX_FAILED_MAGIC_LINK_ATTEMPTS {
            let guess = invalidation
                .get_and_invalidate_magic_link_code(&email, &wrong_code(&code))
                .await?;
            assert!(matches!(
                guess,
                mycelium_base::entities::FetchResponseKind::NotFound(_)
            ));
        }

        // The right code no longer works: the record was discarded
        let verified = invalidation
            .get_and_invalidate_magic_link_code(&email, &code)
            .await?;
        assert!(matches!(
            verified,
            mycelium_base::entities::FetchResponseKind::NotFound(_)
        ));

        Ok(())
    }

    #[tokio::test]
    async fn magic_link_code_still_works_below_max_failed_attempts(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let registration = TokenRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let invalidation = TokenInvalidationSqlDbRepository {
            db_config: db.provider.clone(),
        };

        let email = Email::from_string("owner@acme.test".into())?;
        let code =
            open_magic_link(&registration, &invalidation, &email).await?;

        for _ in 1..MAX_FAILED_MAGIC_LINK_ATTEMPTS {
            invalidation
                .get_and_invalidate_magic_link_code(&email, &wrong_code(&code))
                .await?;
        }

        let verified = invalidation
            .get_and_invalidate_magic_link_code(&email, &code)
            .await?;
        assert!(matches!(
            verified,
            mycelium_base::entities::FetchResponseKind::Found(())
        ));

        Ok(())
    }

    #[tokio::test]
    async fn magic_link_failed_attempts_count_on_every_active_code(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let registration = TokenRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let invalidation = TokenInvalidationSqlDbRepository {
            db_config: db.provider.clone(),
        };

        let email = Email::from_string("owner@acme.test".into())?;
        let first =
            open_magic_link(&registration, &invalidation, &email).await?;
        let second =
            open_magic_link(&registration, &invalidation, &email).await?;

        // A guess that matches neither code
        let guess = (0..1_000_000)
            .map(|n| format!("{:06}", n))
            .find(|candidate| *candidate != first && *candidate != second)
            .unwrap();

        for _ in 0..MAX_FAILED_MAGIC_LINK_ATTEMPTS {
            invalidation
                .get_and_invalidate_magic_link_code(&email, &guess)
                .await?;
        }

        // Opening more links does not grant more guesses: both are gone
        for code in [first, second] {
            let verified = invalidation
                .get_and_invalidate_magic_link_code(&email, &code)
                .await?;
            assert!(matches!(
                verified,
                mycelium_base::entities::FetchResponseKind::NotFound(_)
            ));
        }

        Ok(())
    }
}
