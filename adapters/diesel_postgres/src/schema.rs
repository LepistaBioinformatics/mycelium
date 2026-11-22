// @generated automatically by Diesel CLI.

diesel::table! {
    account (id) {
        id -> Uuid,
        #[max_length = 256]
        name -> Varchar,
        created -> Timestamptz,
        created_by -> Nullable<Jsonb>,
        updated -> Nullable<Timestamptz>,
        updated_by -> Nullable<Jsonb>,
        is_active -> Bool,
        is_checked -> Bool,
        is_archived -> Bool,
        is_deleted -> Bool,
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
    federation_assertion_cache (link_id, kind) {
        link_id -> Uuid,
        peer_id -> Uuid,
        #[max_length = 64]
        kind -> Varchar,
        jws -> Text,
        payload -> Jsonb,
        iat -> Timestamptz,
        exp -> Timestamptz,
        received_at -> Timestamptz,
    }
}

diesel::table! {
    federation_grant_translation (id) {
        id -> Uuid,
        peer_id -> Uuid,
        issuer_tenant_id -> Uuid,
        #[max_length = 140]
        issuer_role -> Varchar,
        issuer_perm -> Int4,
        local_account_id -> Uuid,
        local_guest_role_id -> Uuid,
        local_perm -> Int4,
        enabled -> Bool,
        created -> Timestamptz,
        created_by -> Nullable<Jsonb>,
        updated -> Nullable<Timestamptz>,
        updated_by -> Nullable<Jsonb>,
    }
}

diesel::table! {
    federation_identity_link (id) {
        id -> Uuid,
        local_user_id -> Uuid,
        #[max_length = 140]
        local_email -> Varchar,
        peer_id -> Uuid,
        #[max_length = 64]
        peer_subject_ref -> Nullable<Varchar>,
        #[max_length = 64]
        local_subject_ref -> Varchar,
        #[max_length = 16]
        status -> Varchar,
        created -> Timestamptz,
        established_at -> Nullable<Timestamptz>,
        revoked_at -> Nullable<Timestamptz>,
        revoked_by -> Nullable<Jsonb>,
    }
}

diesel::table! {
    federation_link_consent (id) {
        id -> Uuid,
        peer_id -> Uuid,
        peer_request_id -> Uuid,
        #[max_length = 140]
        peer_email -> Varchar,
        #[max_length = 2048]
        return_url -> Varchar,
        local_user_id -> Nullable<Uuid>,
        created -> Timestamptz,
        expires_at -> Timestamptz,
        consumed_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    federation_link_request (id) {
        id -> Uuid,
        local_user_id -> Uuid,
        peer_id -> Uuid,
        #[max_length = 64]
        secret_hash -> Varchar,
        created -> Timestamptz,
        expires_at -> Timestamptz,
        redirected_at -> Nullable<Timestamptz>,
        consumed_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    federation_namespace_claim (foreign_id) {
        foreign_id -> Uuid,
        peer_id -> Uuid,
        #[max_length = 16]
        kind -> Varchar,
        first_seen_at -> Timestamptz,
    }
}

diesel::table! {
    federation_outbox (id) {
        id -> Uuid,
        peer_id -> Uuid,
        #[max_length = 64]
        subject_ref -> Varchar,
        #[max_length = 32]
        reason -> Varchar,
        issued_at -> Timestamptz,
        attempts -> Int4,
        next_attempt_at -> Timestamptz,
        #[max_length = 16]
        status -> Varchar,
        last_error -> Nullable<Text>,
    }
}

diesel::table! {
    federation_peer (id) {
        id -> Uuid,
        instance_id -> Uuid,
        #[max_length = 2048]
        base_url -> Varchar,
        #[max_length = 255]
        display_name -> Nullable<Varchar>,
        #[max_length = 32]
        status -> Varchar,
        consume_kinds -> Jsonb,
        serve_kinds -> Jsonb,
        negotiated_kinds -> Jsonb,
        policy -> Jsonb,
        last_contact_at -> Nullable<Timestamptz>,
        failure_count -> Int4,
        created -> Timestamptz,
        created_by -> Nullable<Jsonb>,
        updated -> Nullable<Timestamptz>,
        updated_by -> Nullable<Jsonb>,
    }
}

diesel::table! {
    federation_peer_key (peer_id, kid) {
        peer_id -> Uuid,
        #[max_length = 16]
        kid -> Varchar,
        #[max_length = 16]
        alg -> Varchar,
        #[max_length = 64]
        public_key -> Varchar,
        fetched_at -> Timestamptz,
        expires_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    federation_request_nonce (jti) {
        #[max_length = 128]
        jti -> Varchar,
        peer_id -> Uuid,
        expires_at -> Timestamptz,
    }
}

diesel::table! {
    federation_tombstone (link_id) {
        link_id -> Uuid,
        peer_id -> Uuid,
        #[max_length = 32]
        reason -> Varchar,
        issued_at -> Timestamptz,
        received_at -> Timestamptz,
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
        permit_flags -> Nullable<Array<Text>>,
        deny_flags -> Nullable<Array<Text>>,
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
        encrypted_dek -> Nullable<Text>,
        kek_version -> Integer,
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
        created_by -> Nullable<Jsonb>,
        updated -> Nullable<Timestamptz>,
        updated_by -> Nullable<Jsonb>,
        secret -> Nullable<Jsonb>,
        #[max_length = 255]
        trigger -> Varchar,
        method -> Nullable<Varchar>,
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
        claimed_at -> Nullable<Timestamptz>,
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

diesel::table! {
    instance_settings (key) {
        #[max_length = 255]
        key -> Varchar,
        value -> Jsonb,
        created_by -> Nullable<Jsonb>,
        updated_by -> Nullable<Jsonb>,
        created -> Timestamptz,
        updated -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    resource_audit_log (id) {
        id -> Uuid,
        resource_type -> Text,
        resource_id -> Uuid,
        tenant_id -> Nullable<Uuid>,
        event -> Text,
        performed_by -> Jsonb,
        metadata -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    notification_dispatch (id) {
        id -> Uuid,
        sender_tag -> Varchar,
        channel -> Varchar,
        connection_string_id -> Int4,
        account_id -> Uuid,
        tenant_id -> Nullable<Uuid>,
        recipient -> Varchar,
        template -> Varchar,
        locale -> Varchar,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(account -> tenant (tenant_id));
diesel::joinable!(account_tag -> account (account_id));
diesel::joinable!(federation_assertion_cache -> federation_identity_link (link_id));
diesel::joinable!(federation_assertion_cache -> federation_peer (peer_id));
diesel::joinable!(federation_grant_translation -> account (local_account_id));
diesel::joinable!(federation_grant_translation -> federation_peer (peer_id));
diesel::joinable!(federation_grant_translation -> guest_role (local_guest_role_id));
diesel::joinable!(federation_identity_link -> federation_peer (peer_id));
diesel::joinable!(federation_identity_link -> user (local_user_id));
diesel::joinable!(federation_link_consent -> federation_peer (peer_id));
diesel::joinable!(federation_link_consent -> user (local_user_id));
diesel::joinable!(federation_link_request -> federation_peer (peer_id));
diesel::joinable!(federation_link_request -> user (local_user_id));
diesel::joinable!(federation_namespace_claim -> federation_peer (peer_id));
diesel::joinable!(federation_outbox -> federation_peer (peer_id));
diesel::joinable!(federation_peer_key -> federation_peer (peer_id));
diesel::joinable!(federation_request_nonce -> federation_peer (peer_id));
diesel::joinable!(federation_tombstone -> federation_identity_link (link_id));
diesel::joinable!(federation_tombstone -> federation_peer (peer_id));
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
    federation_assertion_cache,
    federation_grant_translation,
    federation_identity_link,
    federation_link_consent,
    federation_link_request,
    federation_namespace_claim,
    federation_outbox,
    federation_peer,
    federation_peer_key,
    federation_request_nonce,
    federation_tombstone,
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
