// SQLite mirror of the Postgres `schema.rs`. Postgres-only SQL types are mapped
// to SQLite-compatible ones (see migrations_sqlite):
//   Uuid -> Text, Jsonb -> Text, Array<*> -> Text, Timestamptz -> Text.
// Column names and order match the Postgres schema so the repository layer maps
// by name via `Selectable`.

diesel::table! {
    account (id) {
        id -> Text,
        name -> Text,
        created -> Text,
        created_by -> Nullable<Text>,
        updated -> Nullable<Text>,
        updated_by -> Nullable<Text>,
        is_active -> Bool,
        is_checked -> Bool,
        is_archived -> Bool,
        is_deleted -> Bool,
        is_default -> Bool,
        slug -> Text,
        account_type -> Text,
        tenant_id -> Nullable<Text>,
        meta -> Nullable<Text>,
    }
}

diesel::table! {
    account_tag (id) {
        id -> Text,
        value -> Text,
        meta -> Nullable<Text>,
        account_id -> Text,
    }
}

diesel::table! {
    error_code (prefix, code) {
        code -> Integer,
        prefix -> Text,
        message -> Text,
        details -> Nullable<Text>,
        is_internal -> Bool,
        is_native -> Bool,
    }
}

diesel::table! {
    guest_role (id) {
        id -> Text,
        name -> Text,
        description -> Nullable<Text>,
        permission -> Integer,
        slug -> Text,
        system -> Bool,
        created -> Text,
        updated -> Nullable<Text>,
    }
}

diesel::table! {
    guest_role_children (parent_id, child_role_id) {
        parent_id -> Text,
        child_role_id -> Text,
        created_by -> Text,
        created -> Text,
        updated -> Nullable<Text>,
    }
}

diesel::table! {
    guest_user (id) {
        id -> Text,
        email -> Text,
        guest_role_id -> Text,
        created -> Text,
        updated -> Nullable<Text>,
        was_verified -> Bool,
    }
}

diesel::table! {
    guest_user_on_account (guest_user_id, account_id) {
        guest_user_id -> Text,
        account_id -> Text,
        created -> Text,
        permit_flags -> Nullable<Text>,
        deny_flags -> Nullable<Text>,
    }
}

diesel::table! {
    identity_provider (user_id) {
        name -> Nullable<Text>,
        password_hash -> Nullable<Text>,
        user_id -> Text,
    }
}

diesel::table! {
    manager_account_on_tenant (id) {
        id -> Text,
        tenant_id -> Text,
        account_id -> Text,
        created -> Text,
        updated -> Nullable<Text>,
    }
}

diesel::table! {
    owner_on_tenant (id) {
        id -> Text,
        tenant_id -> Text,
        owner_id -> Text,
        guest_by -> Text,
        created -> Text,
        updated -> Nullable<Text>,
    }
}

diesel::table! {
    tenant (id) {
        id -> Text,
        name -> Text,
        description -> Nullable<Text>,
        meta -> Nullable<Text>,
        status -> Nullable<Text>,
        created -> Text,
        updated -> Nullable<Text>,
        encrypted_dek -> Nullable<Text>,
        kek_version -> Integer,
    }
}

diesel::table! {
    tenant_tag (id) {
        id -> Text,
        value -> Text,
        meta -> Nullable<Text>,
        tenant_id -> Text,
    }
}

diesel::table! {
    token (id) {
        id -> Integer,
        meta -> Text,
        expiration -> Text,
    }
}

diesel::table! {
    user (id) {
        id -> Text,
        username -> Text,
        email -> Text,
        first_name -> Text,
        last_name -> Text,
        is_active -> Bool,
        created -> Text,
        updated -> Nullable<Text>,
        account_id -> Nullable<Text>,
        is_principal -> Bool,
        mfa -> Nullable<Text>,
    }
}

diesel::table! {
    webhook (id) {
        id -> Text,
        name -> Text,
        description -> Nullable<Text>,
        url -> Text,
        is_active -> Bool,
        created -> Text,
        created_by -> Nullable<Text>,
        updated -> Nullable<Text>,
        updated_by -> Nullable<Text>,
        secret -> Nullable<Text>,
        trigger -> Text,
        method -> Nullable<Text>,
    }
}

diesel::table! {
    webhook_execution (id) {
        id -> Text,
        payload -> Text,
        payload_id -> Text,
        trigger -> Text,
        encrypted -> Nullable<Bool>,
        attempts -> Integer,
        created -> Text,
        attempted -> Nullable<Text>,
        status -> Nullable<Text>,
        propagations -> Nullable<Text>,
    }
}

diesel::table! {
    message_queue (id) {
        id -> Text,
        message -> Text,
        created -> Text,
        attempted -> Nullable<Text>,
        status -> Text,
        attempts -> Integer,
        error -> Nullable<Text>,
    }
}

diesel::table! {
    healthcheck_logs (service_id, checked_at) {
        service_id -> Text,
        service_name -> Text,
        checked_at -> Text,
        status_code -> Integer,
        response_time_ms -> Integer,
        dns_resolved_ip -> Nullable<Text>,
        response_body -> Nullable<Text>,
        error_message -> Nullable<Text>,
        headers -> Nullable<Text>,
        content_type -> Nullable<Text>,
        response_size_bytes -> Nullable<Integer>,
        retry_count -> Nullable<Integer>,
        timeout_occurred -> Nullable<Bool>,
    }
}

diesel::joinable!(account -> tenant (tenant_id));
diesel::joinable!(account_tag -> account (account_id));
diesel::joinable!(guest_user -> guest_role (guest_role_id));
diesel::joinable!(guest_user_on_account -> account (account_id));
diesel::joinable!(guest_user_on_account -> guest_user (guest_user_id));
diesel::joinable!(guest_role_children -> guest_role (child_role_id));
diesel::joinable!(identity_provider -> user (user_id));
diesel::joinable!(manager_account_on_tenant -> account (account_id));
diesel::joinable!(manager_account_on_tenant -> tenant (tenant_id));
diesel::joinable!(owner_on_tenant -> tenant (tenant_id));
diesel::joinable!(tenant_tag -> tenant (tenant_id));
diesel::joinable!(user -> account (account_id));

diesel::allow_tables_to_appear_in_same_query!(
    account,
    account_tag,
    error_code,
    guest_role,
    guest_role_children,
    guest_user,
    guest_user_on_account,
    healthcheck_logs,
    identity_provider,
    manager_account_on_tenant,
    owner_on_tenant,
    tenant,
    tenant_tag,
    token,
    user,
    webhook,
);
