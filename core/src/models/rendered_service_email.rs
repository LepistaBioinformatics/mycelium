/// Subject and body of a service email, rendered from a service template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedServiceEmail {
    pub subject: String,
    pub body: String,

    /// Locale the templates were actually taken from.
    pub locale: String,
}
