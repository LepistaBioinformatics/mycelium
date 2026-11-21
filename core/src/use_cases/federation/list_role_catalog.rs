use crate::domain::{
    dtos::{federation::RoleCatalogEntry, guest_role::GuestRole},
    entities::GuestRoleFetching,
};

use mycelium_base::{
    dtos::Children, entities::FetchManyResponseKind,
    utils::errors::MappedErrors,
};
use std::collections::HashMap;
use uuid::Uuid;

/// Roles read per page while building the catalog
const PAGE_SIZE: i32 = 100;

/// Every guest role of this instance as its role catalog (D-10, FED-55)
#[tracing::instrument(name = "list_role_catalog", skip_all)]
pub async fn list_role_catalog(
    guest_role_fetching_repo: Box<&dyn GuestRoleFetching>,
) -> Result<Vec<RoleCatalogEntry>, MappedErrors> {
    let roles = every_guest_role(*guest_role_fetching_repo).await?;
    let slugs: HashMap<Uuid, String> = roles
        .iter()
        .filter_map(|role| role.id.map(|id| (id, role.slug.clone())))
        .collect();

    Ok(roles.into_iter().map(|role| entry(role, &slugs)).collect())
}

async fn every_guest_role(
    repo: &dyn GuestRoleFetching,
) -> Result<Vec<GuestRole>, MappedErrors> {
    let mut roles = Vec::new();

    loop {
        let (records, total) = read_page(repo, roles.len()).await?;
        let read = records.len();
        roles.extend(records);

        if read == 0 || roles.len() >= total {
            return Ok(roles);
        }
    }
}

/// One page of roles from `skip`, with the total the repository reports
async fn read_page(
    repo: &dyn GuestRoleFetching,
    skip: usize,
) -> Result<(Vec<GuestRole>, usize), MappedErrors> {
    let page = repo
        .list(None, None, None, Some(PAGE_SIZE), Some(skip as i32))
        .await?;

    Ok(match page {
        FetchManyResponseKind::FoundPaginated { records, count, .. } => {
            (records, count as usize)
        }
        FetchManyResponseKind::Found(records) => {
            let total = records.len();
            (records, total)
        }
        FetchManyResponseKind::NotFound => (vec![], 0),
    })
}

fn entry(role: GuestRole, slugs: &HashMap<Uuid, String>) -> RoleCatalogEntry {
    let children = match role.children {
        Some(Children::Ids(ids)) => {
            ids.iter().filter_map(|id| slugs.get(id).cloned()).collect()
        }
        Some(Children::Records(records)) => {
            records.into_iter().map(|child| child.slug).collect()
        }
        None => vec![],
    };

    RoleCatalogEntry {
        slug: role.slug,
        name: role.name,
        description: role.description,
        permission: role.permission,
        system: role.system,
        children,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::guest_role::Permission;

    use async_trait::async_trait;
    use mycelium_base::entities::FetchResponseKind;

    /// Serves `roles` in pages, as the Postgres adapter does
    struct Paged(Vec<GuestRole>);

    #[async_trait]
    impl GuestRoleFetching for Paged {
        async fn get(
            &self,
            _: Uuid,
        ) -> Result<FetchResponseKind<GuestRole, Uuid>, MappedErrors> {
            unreachable!()
        }

        async fn get_parent_by_child_id(
            &self,
            _: Uuid,
        ) -> Result<FetchResponseKind<GuestRole, Uuid>, MappedErrors> {
            unreachable!()
        }

        async fn list(
            &self,
            _: Option<String>,
            _: Option<String>,
            _: Option<bool>,
            page_size: Option<i32>,
            skip: Option<i32>,
        ) -> Result<FetchManyResponseKind<GuestRole>, MappedErrors> {
            let skip = skip.unwrap_or(0) as usize;
            let size = page_size.unwrap_or(10) as usize;

            Ok(FetchManyResponseKind::FoundPaginated {
                count: self.0.len() as i64,
                skip: Some(skip as i64),
                size: Some(size as i64),
                records: self.0.iter().skip(skip).take(size).cloned().collect(),
            })
        }
    }

    fn role(index: usize, children: Option<Vec<Uuid>>) -> GuestRole {
        let mut role = GuestRole::new(
            Some(Uuid::from_u128(index as u128 + 1)),
            format!("Role {index}"),
            Some(format!("Role number {index}")),
            Permission::Read,
            children.map(Children::Ids),
            index == 0,
        );
        role.slug = format!("role-{index}");
        role
    }

    #[tokio::test]
    async fn reads_every_page_and_names_children_by_slug(
    ) -> Result<(), MappedErrors> {
        let mut roles: Vec<GuestRole> =
            (1..=250).map(|i| role(i, None)).collect();
        roles.insert(
            0,
            role(0, Some(vec![Uuid::from_u128(2), Uuid::from_u128(3)])),
        );
        let paged = Paged(roles);

        let catalog =
            list_role_catalog(Box::new(&paged as &dyn GuestRoleFetching))
                .await?;

        assert_eq!(catalog.len(), 251);
        assert_eq!(catalog[0].slug, "role-0");
        assert!(catalog[0].system);
        assert_eq!(catalog[0].children, ["role-1", "role-2"]);
        assert!(catalog[1].children.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn the_catalog_carries_no_ids() -> Result<(), MappedErrors> {
        let paged =
            Paged(vec![role(0, Some(vec![Uuid::from_u128(2)])), role(1, None)]);

        let catalog =
            list_role_catalog(Box::new(&paged as &dyn GuestRoleFetching))
                .await?;
        let json = serde_json::to_string(&catalog).unwrap();

        assert!(!json.contains(&Uuid::from_u128(1).to_string()));
        assert!(!json.contains(&Uuid::from_u128(2).to_string()));
        assert!(!json.contains("Id\""));

        Ok(())
    }
}
