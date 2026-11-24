//! Live Postgres tests of the account soft delete (#246)

use super::AccountDeletionSqlDbRepository;
use crate::models::config::{DbPool, DbPoolProvider};

use diesel::{
    connection::SimpleConnection,
    r2d2::{ConnectionManager, Pool},
    sql_query,
    sql_types::{Bool, Jsonb, Text, Uuid as SqlUuid},
    PgConnection, QueryableByName, RunQueryDsl,
};
use myc_core::domain::{
    dtos::{account_type::AccountType, related_accounts::RelatedAccounts},
    entities::AccountDeletion,
};
use mycelium_base::entities::DeletionResponseKind;
use std::sync::{Arc, Once};
use uuid::Uuid;

/// Same variable as the other live tests of this crate. Without it every
/// test below reports the skip and passes.
const DATABASE_URL_VAR: &str = "MYC_TEST_DATABASE_URL";

/// Own schema, so these tests never race the other live tests.
const SCHEMA: &str = "myc_account_deletion_test";

/// The tables the soft delete writes, as `up.sql` creates them.
const TABLES: &str = r#"
    DROP TABLE IF EXISTS account, "user", account_tag,
        guest_user_on_account, manager_account_on_tenant;
    CREATE TABLE account (
        id UUID DEFAULT gen_random_uuid(),
        name VARCHAR(256) NOT NULL,
        slug VARCHAR(256) NOT NULL,
        meta JSONB,
        account_type JSONB DEFAULT '{}'::JSONB,
        created TIMESTAMPTZ DEFAULT now(),
        created_by JSONB DEFAULT '{}'::JSONB,
        updated TIMESTAMPTZ DEFAULT NULL,
        updated_by JSONB DEFAULT '{}'::JSONB,
        is_active BOOLEAN DEFAULT TRUE,
        is_checked BOOLEAN DEFAULT FALSE,
        is_archived BOOLEAN DEFAULT FALSE,
        is_deleted BOOLEAN DEFAULT FALSE,
        is_default BOOLEAN DEFAULT FALSE,
        tenant_id UUID DEFAULT NULL
    );
    CREATE TABLE "user" (
        id UUID DEFAULT gen_random_uuid(),
        username VARCHAR(140) NOT NULL,
        email VARCHAR(140) NOT NULL,
        first_name VARCHAR(140) NOT NULL,
        last_name VARCHAR(140) NOT NULL,
        is_active BOOLEAN DEFAULT TRUE,
        is_principal BOOLEAN DEFAULT FALSE,
        created TIMESTAMPTZ DEFAULT now(),
        updated TIMESTAMPTZ DEFAULT NULL,
        mfa JSONB,
        account_id UUID DEFAULT NULL
    );
    CREATE TABLE account_tag (
        id UUID DEFAULT gen_random_uuid(),
        value VARCHAR(64) NOT NULL,
        meta JSONB,
        account_id UUID NOT NULL
    );
    CREATE TABLE guest_user_on_account (
        guest_user_id UUID NOT NULL,
        account_id UUID NOT NULL,
        created TIMESTAMPTZ DEFAULT now(),
        permit_flags JSONB[],
        deny_flags JSONB[]
    );
    CREATE TABLE manager_account_on_tenant (
        id UUID DEFAULT gen_random_uuid(),
        tenant_id UUID NOT NULL,
        account_id UUID NOT NULL,
        created TIMESTAMPTZ DEFAULT now(),
        updated TIMESTAMPTZ DEFAULT NULL
    );"#;

static PREPARE: Once = Once::new();

struct TestPool(DbPool);

impl DbPoolProvider for TestPool {
    fn get_pool(&self) -> DbPool {
        self.0.clone()
    }
}

fn live_pool() -> Option<DbPool> {
    let Ok(url) = std::env::var(DATABASE_URL_VAR) else {
        eprintln!("skipping: set {DATABASE_URL_VAR} to run the live tests");
        return None;
    };

    let separator = if url.contains('?') { '&' } else { '?' };
    let url = format!("{url}{separator}options=-csearch_path%3D{SCHEMA}");
    let pool = Pool::builder()
        .max_size(4)
        .build(ConnectionManager::<PgConnection>::new(url))
        .expect("failed to build the test pool");

    PREPARE.call_once(|| {
        pool.get()
            .expect("failed to take a test connection")
            .batch_execute(&format!(
                "CREATE SCHEMA IF NOT EXISTS {SCHEMA}; {TABLES}"
            ))
            .expect("failed to prepare the test schema");
    });

    Some(pool)
}

/// A user account with its user, a tag, a guest and a tenant management link
fn seed_account(pool: &DbPool, with_user: bool) -> Uuid {
    let conn = &mut pool.get().unwrap();
    let account_id = Uuid::new_v4();
    let account_type = serde_json::to_value(AccountType::User).unwrap();

    sql_query(
        "INSERT INTO account (id, name, slug, account_type)
         VALUES ($1, 'Account 01', 'account-01', $2)",
    )
    .bind::<SqlUuid, _>(account_id)
    .bind::<Jsonb, _>(account_type)
    .execute(conn)
    .unwrap();

    let links = [
        "INSERT INTO account_tag (value, account_id) VALUES ('tag-01', $1)",
        "INSERT INTO guest_user_on_account (guest_user_id, account_id)
         VALUES (gen_random_uuid(), $1)",
        "INSERT INTO manager_account_on_tenant (tenant_id, account_id)
         VALUES (gen_random_uuid(), $1)",
    ];
    let user = "INSERT INTO \"user\"
        (username, email, first_name, last_name, is_principal, account_id)
        VALUES ('user-01', 'user-01@example.com', 'User', 'One', true, $1)";

    for statement in links.iter().chain(with_user.then_some(&user)) {
        sql_query(*statement)
            .bind::<SqlUuid, _>(account_id)
            .execute(conn)
            .unwrap();
    }

    account_id
}

#[derive(QueryableByName, Debug, PartialEq)]
struct Standing {
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Bool)]
    is_active: bool,
    #[diesel(sql_type = Bool)]
    is_deleted: bool,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    leftovers: i64,
}

fn standing(pool: &DbPool, account_id: Uuid) -> Standing {
    sql_query(
        r#"SELECT a.name, a.is_active, a.is_deleted,
              (SELECT count(*) FROM account_tag WHERE account_id = $1)
            + (SELECT count(*) FROM guest_user_on_account WHERE account_id = $1)
            + (SELECT count(*) FROM manager_account_on_tenant
               WHERE account_id = $1)
            + (SELECT count(*) FROM "user"
               WHERE account_id = $1 AND (is_active OR email NOT LIKE '%-deleted'))
              AS leftovers
           FROM account a WHERE a.id = $1"#,
    )
    .bind::<SqlUuid, _>(account_id)
    .get_result(&mut pool.get().unwrap())
    .unwrap()
}

async fn soft_delete(
    pool: &DbPool,
    account_id: Uuid,
) -> DeletionResponseKind<Uuid> {
    let repository = AccountDeletionSqlDbRepository {
        db_config: Arc::new(TestPool(pool.clone())),
    };

    repository
        .soft_delete_account(
            account_id,
            AccountType::User,
            RelatedAccounts::AllowedAccounts(vec![account_id]),
        )
        .await
        .unwrap()
}

fn assert_soft_deleted(pool: &DbPool, account_id: Uuid) {
    assert_eq!(
        standing(pool, account_id),
        Standing {
            name: format!("{account_id}-deleted"),
            is_active: false,
            is_deleted: true,
            leftovers: 0,
        }
    );
}

#[tokio::test]
async fn soft_deletes_an_account_with_its_user() {
    let Some(pool) = live_pool() else { return };
    let account_id = seed_account(&pool, true);

    let response = soft_delete(&pool, account_id).await;

    assert!(matches!(response, DeletionResponseKind::Deleted));
    assert_soft_deleted(&pool, account_id);
}

#[tokio::test]
async fn soft_deletes_an_account_without_a_user() {
    let Some(pool) = live_pool() else { return };
    let account_id = seed_account(&pool, false);

    let response = soft_delete(&pool, account_id).await;

    assert!(matches!(response, DeletionResponseKind::Deleted));
    assert_soft_deleted(&pool, account_id);
}

#[tokio::test]
async fn reports_an_unknown_account_as_not_deleted() {
    let Some(pool) = live_pool() else { return };

    let response = soft_delete(&pool, Uuid::new_v4()).await;

    assert!(matches!(response, DeletionResponseKind::NotDeleted(..)));
}
