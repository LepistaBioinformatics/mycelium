diesel::table! {
    tenant (id) {
        id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        meta -> Nullable<Jsonb>,
        status -> Array<Jsonb>,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    owner_on_tenant (id) {
        id -> Uuid,
        tenant_id -> Uuid,
        owner_id -> Uuid,
        guest_by -> Varchar,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
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
    tenant_tag (id) {
        id -> Uuid,
        value -> Varchar,
        meta -> Nullable<Jsonb>,
        tenant_id -> Uuid,
    }
}

diesel::table! {
    user (id) {
        id -> Uuid,
        username -> Varchar,
        email -> Varchar,
        first_name -> Varchar,
        last_name -> Varchar,
        is_active -> Bool,
        is_principal -> Bool,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
        mfa -> Nullable<Jsonb>,
        account_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    identity_provider (user_id) {
        user_id -> Uuid,
        name -> Nullable<Varchar>,
        password_hash -> Nullable<Varchar>,
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
    guest_role (id) {
        id -> Uuid,
        name -> Varchar,
        slug -> Varchar,
        description -> Nullable<Varchar>,
        permission -> Int4,
    }
}

diesel::table! {
    guest_role_children (parent_id, child_role_id) {
        parent_id -> Uuid,
        child_role_id -> Uuid,
    }
}

diesel::table! {
    guest_user (id) {
        id -> Uuid,
        email -> Varchar,
        guest_role_id -> Uuid,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
        was_verified -> Bool,
    }
}

diesel::table! {
    account (id) {
        id -> Uuid,
        name -> Varchar,
        slug -> Varchar,
        meta -> Nullable<Jsonb>,
        account_type -> Jsonb,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
        is_active -> Bool,
        is_checked -> Bool,
        is_archived -> Bool,
        is_default -> Bool,
        tenant_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    account_tag (id) {
        id -> Uuid,
        value -> Varchar,
        meta -> Nullable<Jsonb>,
        account_id -> Uuid,
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
    error_code (prefix, code) {
        code -> Int4,
        prefix -> Varchar,
        message -> Varchar,
        details -> Nullable<Varchar>,
        is_internal -> Bool,
        is_native -> Bool,
    }
}

diesel::table! {
    webhook (id) {
        id -> Uuid,
        name -> Varchar,
        description -> Nullable<Varchar>,
        url -> Varchar,
        trigger -> Varchar,
        is_active -> Bool,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
        secret -> Nullable<Jsonb>,
    }
}

diesel::joinable!(owner_on_tenant -> tenant (tenant_id));
diesel::joinable!(owner_on_tenant -> user (owner_id));
diesel::joinable!(manager_account_on_tenant -> tenant (tenant_id));
diesel::joinable!(manager_account_on_tenant -> account (account_id));
diesel::joinable!(tenant_tag -> tenant (tenant_id));
diesel::joinable!(guest_user -> guest_role (guest_role_id));
diesel::joinable!(account -> tenant (tenant_id));
diesel::joinable!(account_tag -> account (account_id));
diesel::joinable!(guest_user_on_account -> account (account_id));
diesel::joinable!(guest_user_on_account -> guest_user (guest_user_id));
diesel::joinable!(identity_provider -> user (user_id));
diesel::joinable!(user -> account (account_id));

diesel::allow_tables_to_appear_in_same_query!(
    tenant,
    owner_on_tenant,
    manager_account_on_tenant,
    tenant_tag,
    user,
    identity_provider,
    token,
    guest_role,
    guest_role_children,
    guest_user,
    account,
    account_tag,
    guest_user_on_account,
    error_code,
    webhook,
);
