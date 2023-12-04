use crate::endpoints::shared::{build_scoped_path, UrlGroup, UrlScopes};

use myc_core::domain::actors::DefaultActor;

pub fn build_actor_context(actor: DefaultActor, group: UrlGroup) -> String {
    format!(
        "{}/{}/{}",
        build_scoped_path(UrlScopes::Standards),
        actor,
        group
    )
}
