use diesel::{
    dsl::{sql, AsExprOf},
    expression::{SqlLiteral, UncheckedBind},
    sql_types::{Bool, Jsonb},
};
use serde::Serialize;
use serde_json::Value as JsonValue;

pub(super) type AccountTypeContainment =
    UncheckedBind<SqlLiteral<Bool>, AsExprOf<JsonValue, Jsonb>>;

/// Builds the `account_type @> <account type>` filter.
///
/// The account type is sent as a bind parameter: variants such as
/// `RoleAssociated` carry free-form strings, so the serialized document must
/// never be interpolated into the SQL text.
pub(super) fn account_type_containment<T: Serialize>(
    account_type: &T,
) -> Result<AccountTypeContainment, serde_json::Error> {
    let document = serde_json::to_value(account_type)?;

    Ok(sql::<Bool>("account_type::jsonb @> ").bind::<Jsonb, _>(document))
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::{debug_query, pg::Pg};
    use myc_core::domain::dtos::account_type::AccountType;
    use uuid::Uuid;

    #[test]
    fn account_type_filter_binds_quotes_instead_of_interpolating_them() {
        let injection = "x' OR (SELECT pg_sleep(5)) IS NULL OR '";
        let account_type = AccountType::RoleAssociated {
            tenant_id: Uuid::nil(),
            role_name: injection.to_string(),
            read_role_id: Uuid::nil(),
            write_role_id: Uuid::nil(),
        };
        let filter = account_type_containment(&account_type).unwrap();

        let rendered = debug_query::<Pg, _>(&filter).to_string();
        let (sql_text, binds) = rendered.split_once(" -- binds: ").unwrap();

        assert_eq!(sql_text, "account_type::jsonb @> $1");
        assert!(binds.contains("pg_sleep"));
    }
}
