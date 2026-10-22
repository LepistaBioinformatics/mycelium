use super::*;
use crate::test_support::setup_temp_db;

use chrono::{Duration, Local};
use myc_core::domain::{
    dtos::{
        email::Email,
        hosted_login::{
            HostedLoginCodeAttachment, HostedLoginCodeCheck,
            HostedLoginEmailCode, HostedLoginRequest, HostedLoginSession,
            MAX_FAILED_HOSTED_LOGIN_ATTEMPTS,
            MAX_PENDING_HOSTED_LOGIN_CODES_PER_EMAIL,
        },
    },
    entities::{
        HostedLoginSessionFetching, HostedLoginSessionInvalidation,
        HostedLoginSessionRegistration, HostedLoginSessionUpdating,
    },
};
use mycelium_base::{
    entities::{CreateResponseKind, FetchResponseKind},
    utils::errors::MappedErrors,
};

struct Repos {
    _db: crate::test_support::TempDb,
    registration: HostedLoginSessionRegistrationSqlDbRepository,
    fetching: HostedLoginSessionFetchingSqlDbRepository,
    updating: HostedLoginSessionUpdatingSqlDbRepository,
    invalidation: HostedLoginSessionInvalidationSqlDbRepository,
}

fn repos() -> Repos {
    let db = setup_temp_db();

    Repos {
        registration: HostedLoginSessionRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        },
        fetching: HostedLoginSessionFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        },
        updating: HostedLoginSessionUpdatingSqlDbRepository {
            db_config: db.provider.clone(),
        },
        invalidation: HostedLoginSessionInvalidationSqlDbRepository {
            db_config: db.provider.clone(),
        },
        _db: db,
    }
}

fn email() -> Email {
    Email::from_string("user@example.com".to_string()).unwrap()
}

async fn open_session(
    repos: &Repos,
) -> Result<HostedLoginSession, MappedErrors> {
    let session = HostedLoginSession::new(HostedLoginRequest {
        client_id: "webapp".to_string(),
        redirect_uri: "https://app.example.com/callback".to_string(),
        state: "xyz".to_string(),
        code_challenge: "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
            .to_string(),
    });

    match repos
        .registration
        .create(session, Local::now() + Duration::minutes(15))
        .await?
    {
        CreateResponseKind::Created(session) => Ok(session),
        CreateResponseKind::NotCreated(_, msg) => panic!("{msg}"),
    }
}

/// Open a session and attach a code; returns the session id and the code.
async fn session_with_code(
    repos: &Repos,
) -> Result<(String, String), MappedErrors> {
    let session = open_session(repos).await?;
    let email_code = HostedLoginEmailCode::new(email());
    let code = email_code.code.clone();

    let attachment = repos
        .updating
        .attach_email_code(
            &session.session_id,
            email_code,
            MAX_PENDING_HOSTED_LOGIN_CODES_PER_EMAIL,
        )
        .await?;
    assert_eq!(attachment, HostedLoginCodeAttachment::Attached);

    Ok((session.session_id, code))
}

fn wrong_code(code: &str) -> String {
    format!("{:06}", (code.parse::<u32>().unwrap() + 1) % 1_000_000)
}

async fn check(
    repos: &Repos,
    session_id: &str,
    code: &str,
    expires: chrono::DateTime<Local>,
) -> Result<HostedLoginCodeCheck, MappedErrors> {
    repos
        .invalidation
        .check_code_and_issue_authorization_code(
            session_id,
            code,
            "auth-code".to_string(),
            expires,
        )
        .await
}

#[tokio::test]
async fn session_round_trips() -> Result<(), MappedErrors> {
    let repos = repos();
    let session = open_session(&repos).await?;

    let fetched = repos
        .fetching
        .get_by_session_id(&session.session_id)
        .await?;

    let FetchResponseKind::Found(fetched) = fetched else {
        panic!("expected the session to be found");
    };
    assert_eq!(fetched, session);
    assert!(matches!(
        repos.fetching.get_by_session_id("unknown").await?,
        FetchResponseKind::NotFound(_)
    ));

    Ok(())
}

#[tokio::test]
async fn a_session_receives_a_single_code() -> Result<(), MappedErrors> {
    let repos = repos();
    let (session_id, _) = session_with_code(&repos).await?;

    let second = repos
        .updating
        .attach_email_code(
            &session_id,
            HostedLoginEmailCode::new(email()),
            MAX_PENDING_HOSTED_LOGIN_CODES_PER_EMAIL,
        )
        .await?;

    assert_eq!(second, HostedLoginCodeAttachment::AlreadySent);

    Ok(())
}

#[tokio::test]
async fn pending_codes_per_email_are_capped() -> Result<(), MappedErrors> {
    let repos = repos();

    for _ in 0..MAX_PENDING_HOSTED_LOGIN_CODES_PER_EMAIL {
        session_with_code(&repos).await?;
    }

    let session = open_session(&repos).await?;
    let attachment = repos
        .updating
        .attach_email_code(
            &session.session_id,
            HostedLoginEmailCode::new(email()),
            MAX_PENDING_HOSTED_LOGIN_CODES_PER_EMAIL,
        )
        .await?;

    assert_eq!(attachment, HostedLoginCodeAttachment::EmailCapReached);

    // Another address is not affected
    let other = repos
        .updating
        .attach_email_code(
            &session.session_id,
            HostedLoginEmailCode::new(Email::from_string(
                "other@example.com".to_string(),
            )?),
            MAX_PENDING_HOSTED_LOGIN_CODES_PER_EMAIL,
        )
        .await?;
    assert_eq!(other, HostedLoginCodeAttachment::Attached);

    Ok(())
}

#[tokio::test]
async fn right_code_after_wrong_ones_issues_authorization(
) -> Result<(), MappedErrors> {
    let repos = repos();
    let (session_id, code) = session_with_code(&repos).await?;
    let expires = Local::now() + Duration::seconds(60);

    for _ in 1..MAX_FAILED_HOSTED_LOGIN_ATTEMPTS {
        let rejected =
            check(&repos, &session_id, &wrong_code(&code), expires).await?;
        assert!(matches!(rejected, HostedLoginCodeCheck::Rejected(_)));
    }

    let HostedLoginCodeCheck::Verified(session) =
        check(&repos, &session_id, &code, expires).await?
    else {
        panic!("expected the right code to be verified");
    };

    let authorization = session.authorization.unwrap();
    assert_eq!(authorization.code, "auth-code");
    assert_eq!(authorization.email, email());
    assert!(session.email_code.is_none());

    // The emailed code cannot be used twice
    let reused = check(&repos, &session_id, &code, expires).await?;
    assert_eq!(reused, HostedLoginCodeCheck::NotFound);

    Ok(())
}

#[tokio::test]
async fn session_is_discarded_after_max_failed_attempts(
) -> Result<(), MappedErrors> {
    let repos = repos();
    let (session_id, code) = session_with_code(&repos).await?;
    let expires = Local::now() + Duration::seconds(60);

    for _ in 1..MAX_FAILED_HOSTED_LOGIN_ATTEMPTS {
        check(&repos, &session_id, &wrong_code(&code), expires).await?;
    }

    let last = check(&repos, &session_id, &wrong_code(&code), expires).await?;
    assert_eq!(last, HostedLoginCodeCheck::NotFound);

    // The right code no longer works
    let right = check(&repos, &session_id, &code, expires).await?;
    assert_eq!(right, HostedLoginCodeCheck::NotFound);

    Ok(())
}

#[tokio::test]
async fn session_without_code_cannot_be_checked() -> Result<(), MappedErrors> {
    let repos = repos();
    let session = open_session(&repos).await?;

    let result = check(
        &repos,
        &session.session_id,
        "123456",
        Local::now() + Duration::seconds(60),
    )
    .await?;

    assert_eq!(result, HostedLoginCodeCheck::NotFound);

    Ok(())
}

#[tokio::test]
async fn authorization_code_is_single_use() -> Result<(), MappedErrors> {
    let repos = repos();
    let (session_id, code) = session_with_code(&repos).await?;
    check(
        &repos,
        &session_id,
        &code,
        Local::now() + Duration::seconds(60),
    )
    .await?;

    let first = repos
        .invalidation
        .consume_authorization_code("auth-code")
        .await?;
    let FetchResponseKind::Found(session) = first else {
        panic!("expected the authorization code to be found");
    };
    assert_eq!(session.session_id, session_id);

    let second = repos
        .invalidation
        .consume_authorization_code("auth-code")
        .await?;
    assert!(matches!(second, FetchResponseKind::NotFound(_)));

    Ok(())
}

#[tokio::test]
async fn expired_authorization_code_is_rejected() -> Result<(), MappedErrors> {
    let repos = repos();
    let (session_id, code) = session_with_code(&repos).await?;
    check(
        &repos,
        &session_id,
        &code,
        Local::now() - Duration::seconds(1),
    )
    .await?;

    let consumed = repos
        .invalidation
        .consume_authorization_code("auth-code")
        .await?;

    assert!(matches!(consumed, FetchResponseKind::NotFound(_)));

    Ok(())
}

#[tokio::test]
async fn exhausted_sessions_keep_counting_against_the_email_cap(
) -> Result<(), MappedErrors> {
    let repos = repos();
    let expires = Local::now() + Duration::seconds(60);

    for _ in 0..MAX_PENDING_HOSTED_LOGIN_CODES_PER_EMAIL {
        let (session_id, code) = session_with_code(&repos).await?;

        for _ in 0..MAX_FAILED_HOSTED_LOGIN_ATTEMPTS {
            check(&repos, &session_id, &wrong_code(&code), expires).await?;
        }
    }

    let session = open_session(&repos).await?;
    let attachment = repos
        .updating
        .attach_email_code(
            &session.session_id,
            HostedLoginEmailCode::new(email()),
            MAX_PENDING_HOSTED_LOGIN_CODES_PER_EMAIL,
        )
        .await?;

    assert_eq!(attachment, HostedLoginCodeAttachment::EmailCapReached);

    Ok(())
}
