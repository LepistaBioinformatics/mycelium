// @generated automatically by Diesel CLI.

diesel::table! {
    account (id) {
        #[max_length = 36]
        id -> Text,
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
        account_type -> Json,
        #[max_length = 36]
        tenant_id -> Nullable<Text>,
        meta -> Nullable<Jsonb>,
    }
}

diesel::table! {
    account_tag (id) {
        #[max_length = 36]
        id -> Text,
        #[max_length = 64]
        value -> Varchar,
        meta -> Nullable<Jsonb>,
        #[max_length = 36]
        account_id -> Text,
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
        #[max_length = 36]
        id -> Text,
        #[max_length = 140]
        name -> Varchar,
        #[max_length = 255]
        description -> Nullable<Varchar>,
        permission -> Int4,
        #[max_length = 140]
        slug -> Varchar,
    }
}

diesel::table! {
    guest_role_children (parent_id, child_role_id) {
        #[max_length = 36]
        parent_id -> Text,
        #[max_length = 36]
        child_role_id -> Text,
    }
}

diesel::table! {
    guest_user (id) {
        #[max_length = 36]
        id -> Text,
        #[max_length = 255]
        email -> Varchar,
        #[max_length = 36]
        guest_role_id -> Text,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
        was_verified -> Bool,
    }
}

diesel::table! {
    guest_user_on_account (guest_user_id, account_id) {
        #[max_length = 36]
        guest_user_id -> Text,
        #[max_length = 36]
        account_id -> Text,
        created -> Timestamptz,
    }
}

diesel::table! {
    identity_provider (user_id) {
        #[max_length = 255]
        name -> Nullable<Varchar>,
        #[max_length = 255]
        password_hash -> Nullable<Varchar>,
        #[max_length = 36]
        user_id -> Text,
    }
}

diesel::table! {
    manager_account_on_tenant (id) {
        #[max_length = 36]
        id -> Text,
        #[max_length = 36]
        tenant_id -> Text,
        #[max_length = 36]
        account_id -> Text,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    owner_on_tenant (id) {
        #[max_length = 36]
        id -> Text,
        #[max_length = 36]
        tenant_id -> Text,
        #[max_length = 36]
        owner_id -> Text,
        guest_by -> Text,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    tenant (id) {
        #[max_length = 36]
        id -> Text,
        #[max_length = 255]
        name -> Varchar,
        description -> Nullable<Text>,
        meta -> Nullable<Jsonb>,
        status -> Nullable<Array<Nullable<Jsonb>>>,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    tenant_tag (id) {
        #[max_length = 36]
        id -> Text,
        #[max_length = 64]
        value -> Varchar,
        meta -> Nullable<Jsonb>,
        #[max_length = 36]
        tenant_id -> Text,
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
        #[max_length = 36]
        id -> Text,
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
        #[max_length = 36]
        account_id -> Nullable<Text>,
        is_principal -> Bool,
        mfa -> Nullable<Jsonb>,
    }
}

diesel::table! {
    webhook (id) {
        #[max_length = 36]
        id -> Text,
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

diesel::joinable!(account -> tenant (tenant_id));
diesel::joinable!(account_tag -> account (account_id));
diesel::joinable!(guest_user -> guest_role (guest_role_id));
diesel::joinable!(guest_user_on_account -> account (account_id));
diesel::joinable!(guest_user_on_account -> guest_user (guest_user_id));
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
    identity_provider,
    manager_account_on_tenant,
    owner_on_tenant,
    tenant,
    tenant_tag,
    token,
    user,
    webhook,
);
