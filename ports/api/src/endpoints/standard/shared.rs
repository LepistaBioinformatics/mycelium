use crate::endpoints::shared::{UrlGroup, UrlScopes};

use myc_core::domain::actors::DefaultActor;

pub fn build_actor_context(actor: DefaultActor, group: UrlGroup) -> String {
    group.with_scoped_actor(UrlScopes::Standards, actor)
}
