// @generated automatically by Diesel CLI.

diesel::table! {
    account (id) {
        id -> Uuid,
        #[max_length = 256]
        name -> Varchar,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
        is_active -> Bool,
        is_checked -> Bool,
        is_archived -> Bool,
        is_default -> Bool,
        #[max_length = 256]
        slug -> Varchar,
        account_type -> Jsonb,
        #[max_length = 36]
        tenant_id -> Nullable<Uuid>,
        meta -> Nullable<Jsonb>,
    }
}

diesel::table! {
    account_tag (id) {
        id -> Uuid,
        #[max_length = 64]
        value -> Varchar,
        meta -> Nullable<Jsonb>,
        account_id -> Uuid,
    }
}

diesel::table! {
    error_code (prefix, code) {
        code -> Int4,
        prefix -> Text,
        #[max_length = 255]
        message -> Varchar,
        #[max_length = 255]
        details -> Nullable<Varchar>,
        is_internal -> Bool,
        is_native -> Bool,
    }
}

diesel::table! {
    guest_role (id) {
        id -> Uuid,
        #[max_length = 140]
        name -> Varchar,
        #[max_length = 255]
        description -> Nullable<Varchar>,
        permission -> Int4,
        #[max_length = 140]
        slug -> Varchar,
        system -> Bool,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    guest_role_children (parent_id, child_role_id) {
        parent_id -> Uuid,
        child_role_id -> Uuid,
        created_by -> Uuid,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    guest_user (id) {
        id -> Uuid,
        #[max_length = 255]
        email -> Varchar,
        guest_role_id -> Uuid,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
        was_verified -> Bool,
    }
}

diesel::table! {
    guest_user_on_account (guest_user_id, account_id) {
        guest_user_id -> Uuid,
        account_id -> Uuid,
        created -> Timestamptz,
    }
}

diesel::table! {
    identity_provider (user_id) {
        #[max_length = 255]
        name -> Nullable<Varchar>,
        #[max_length = 255]
        password_hash -> Nullable<Varchar>,
        user_id -> Uuid,
    }
}

diesel::table! {
    manager_account_on_tenant (id) {
        id -> Uuid,
        tenant_id -> Uuid,
        account_id -> Uuid,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    owner_on_tenant (id) {
        id -> Uuid,
        tenant_id -> Uuid,
        owner_id -> Uuid,
        guest_by -> Text,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    tenant (id) {
        id -> Uuid,
        #[max_length = 255]
        name -> Varchar,
        description -> Nullable<Text>,
        meta -> Nullable<Jsonb>,
        status -> Nullable<Array<Jsonb>>,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    tenant_tag (id) {
        id -> Uuid,
        #[max_length = 64]
        value -> Varchar,
        meta -> Nullable<Jsonb>,
        tenant_id -> Uuid,
    }
}

diesel::table! {
    token (id) {
        id -> Int4,
        meta -> Jsonb,
        expiration -> Timestamptz,
    }
}

diesel::table! {
    user (id) {
        id -> Uuid,
        #[max_length = 140]
        username -> Varchar,
        #[max_length = 140]
        email -> Varchar,
        #[max_length = 140]
        first_name -> Varchar,
        #[max_length = 140]
        last_name -> Varchar,
        is_active -> Bool,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
        account_id -> Nullable<Uuid>,
        is_principal -> Bool,
        mfa -> Nullable<Jsonb>,
    }
}

diesel::table! {
    webhook (id) {
        id -> Uuid,
        #[max_length = 140]
        name -> Varchar,
        #[max_length = 255]
        description -> Nullable<Varchar>,
        url -> Text,
        is_active -> Bool,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
        secret -> Nullable<Jsonb>,
        #[max_length = 255]
        trigger -> Varchar,
    }
}

diesel::table! {
    webhook_execution (id) {
        id -> Uuid,
        payload -> Text,
        #[max_length = 255]
        payload_id -> Varchar,
        #[max_length = 255]
        trigger -> Varchar,
        encrypted -> Nullable<Bool>,
        attempts -> Int4,
        created -> Timestamptz,
        attempted -> Nullable<Timestamptz>,
        #[max_length = 100]
        status -> Nullable<Varchar>,
        propagations -> Nullable<Jsonb>,
    }
}

diesel::table! {
    message_queue (id) {
        id -> Uuid,
        message -> Text,
        created -> Timestamptz,
        attempted -> Nullable<Timestamptz>,
        status -> Varchar,
        attempts -> Int4,
        error -> Nullable<Text>,
    }
}

diesel::table! {
    healthcheck_logs (service_id, checked_at) {
        service_id -> Uuid,
        #[max_length = 255]
        service_name -> Varchar,
        checked_at -> Timestamptz,
        status_code -> Int4,
        response_time_ms -> Int4,
        dns_resolved_ip -> Nullable<Text>,
        response_body -> Nullable<Text>,
        error_message -> Nullable<Text>,
        headers -> Nullable<Jsonb>,
        content_type -> Nullable<Text>,
        response_size_bytes -> Nullable<Int4>,
        retry_count -> Nullable<Int4>,
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
diesel::joinable!(owner_on_tenant -> user (owner_id));
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
