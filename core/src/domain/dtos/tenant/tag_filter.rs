/// Filters tenants by one of their tags
///
/// Both criteria apply to the same tag: `value` matches the tag value exactly
/// and `meta` matches one `key: value` pair inside the tag meta.
#[derive(Clone, Debug, Default)]
pub struct TenantTagFilter {
    pub value: Option<String>,
    pub meta: Option<(String, String)>,
}
