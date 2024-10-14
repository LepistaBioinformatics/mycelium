use crate::endpoints::shared::{UrlGroup, UrlScope};

use myc_http_tools::ActorName;

pub fn build_actor_context(actor: ActorName, group: UrlGroup) -> String {
    group.with_scoped_actor(UrlScope::Standards, actor)
}
