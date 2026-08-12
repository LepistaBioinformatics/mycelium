diesel::table! {
    kv_artifact (key) {
        key -> diesel::sql_types::Text,
        value -> diesel::sql_types::Text,
        expires_at -> diesel::sql_types::Timestamptz,
    }
}
