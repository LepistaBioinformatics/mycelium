//! Tell linked peers when a local change removes a subject's access (FED-22)
//!
//! Collect the subjects *before* the change runs (a deletion takes them
//! away), run it, and queue only when it succeeded. The federation
//! dispatcher pushes the queued tombstones. Without the `federation` feature
//! every step is a no-op.

use crate::models::active_backend_modules::SqlAppModule;

use myc_core::domain::dtos::email::Email;
use uuid::Uuid;

/// The emails whose asserted grants a pending change removes
#[derive(Default)]
pub(crate) struct FederatedAccessRevocation {
    #[cfg_attr(not(feature = "federation"), allow(dead_code))]
    emails: Vec<String>,
}

impl FederatedAccessRevocation {
    /// One subject; the email is normalized as stored links hold it
    pub(crate) fn of_email(email: String) -> Self {
        let normalized = Email::from_string(email.clone())
            .map(|email| email.email())
            .unwrap_or(email);

        Self {
            emails: vec![normalized],
        }
    }

    /// The account's own users and its guests
    pub(crate) async fn of_account(
        sql_module: &SqlAppModule,
        account_id: Uuid,
    ) -> Self {
        #[cfg(feature = "federation")]
        return federated::of_account(sql_module, account_id).await;

        #[cfg(not(feature = "federation"))]
        {
            let _ = (sql_module, account_id);
            Self::default()
        }
    }

    /// The guests holding the role
    pub(crate) async fn of_guest_role(
        sql_module: &SqlAppModule,
        guest_role_id: Uuid,
    ) -> Self {
        #[cfg(feature = "federation")]
        return federated::of_guest_role(sql_module, guest_role_id).await;

        #[cfg(not(feature = "federation"))]
        {
            let _ = (sql_module, guest_role_id);
            Self::default()
        }
    }

    /// Queue a tombstone for every peer linked with these subjects
    pub(crate) async fn queue(self, sql_module: &SqlAppModule) {
        #[cfg(feature = "federation")]
        federated::queue(self.emails, sql_module).await;

        #[cfg(not(feature = "federation"))]
        let _ = sql_module;
    }
}

#[cfg(feature = "federation")]
mod federated {
    use super::FederatedAccessRevocation;
    use crate::models::active_backend_modules::SqlAppModule;

    use myc_core::{
        domain::{
            dtos::federation::TombstoneReason,
            entities::RevocationSubjectsFetching,
        },
        use_cases::federation::queue_revocations,
    };
    use mycelium_base::utils::errors::MappedErrors;
    use shaku::HasComponent;
    use uuid::Uuid;

    fn collected(
        emails: Result<Vec<String>, MappedErrors>,
    ) -> FederatedAccessRevocation {
        match emails {
            Ok(emails) => FederatedAccessRevocation { emails },
            Err(err) => {
                tracing::error!(%err, "revocation subjects not read");
                FederatedAccessRevocation::default()
            }
        }
    }

    pub(super) async fn of_account(
        sql_module: &SqlAppModule,
        account_id: Uuid,
    ) -> FederatedAccessRevocation {
        let subjects: &dyn RevocationSubjectsFetching =
            sql_module.resolve_ref();
        collected(subjects.emails_of_account(account_id).await)
    }

    pub(super) async fn of_guest_role(
        sql_module: &SqlAppModule,
        guest_role_id: Uuid,
    ) -> FederatedAccessRevocation {
        let subjects: &dyn RevocationSubjectsFetching =
            sql_module.resolve_ref();
        collected(subjects.emails_of_guest_role(guest_role_id).await)
    }

    pub(super) async fn queue(emails: Vec<String>, sql_module: &SqlAppModule) {
        if let Err(err) = queue_revocations(
            &emails,
            TombstoneReason::GrantsChanged,
            Box::new(sql_module.resolve_ref()),
            Box::new(sql_module.resolve_ref()),
        )
        .await
        {
            tracing::error!(%err, "revocations not queued");
        }
    }
}
