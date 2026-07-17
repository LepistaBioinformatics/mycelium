#![cfg(feature = "local-transport")]

use crate::repositories::render_stub_email_for_terminal::render_stub_email_for_terminal;
use crate::repositories::shared::build_lettre_message;

use async_trait::async_trait;
use lettre::transport::{file::FileTransport, stub::StubTransport};
use lettre::{SmtpTransport, Transport};
use myc_core::domain::{dtos::message::Message, entities::RemoteMessageWrite};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::path::PathBuf;
use uuid::Uuid;

/// The concrete local transport chosen for a standalone build. SMTP takes
/// precedence when configured (SM-R8), so operators can still point a
/// standalone deployment at a real mail server; otherwise mail is written to
/// disk as `.eml` files, falling back to a log-only stub when neither is
/// configured.
pub enum LocalTransportKind {
    Smtp(SmtpTransport),
    File(FileTransport),
    Stub(StubTransport),
}

/// Selection precedence (SM-R8): SMTP if configured, else file transport if
/// a directory was configured, else the stub (log-only) transport.
pub fn select_local_transport(
    smtp: Option<SmtpTransport>,
    file_dir: Option<PathBuf>,
) -> LocalTransportKind {
    if let Some(smtp) = smtp {
        return LocalTransportKind::Smtp(smtp);
    }

    if let Some(dir) = file_dir {
        return LocalTransportKind::File(FileTransport::new(dir));
    }

    LocalTransportKind::Stub(StubTransport::new_ok())
}

#[derive(Component)]
#[shaku(interface = RemoteMessageWrite)]
pub struct LocalTransportMessageSendingRepository {
    pub transport: LocalTransportKind,
}

#[async_trait]
impl RemoteMessageWrite for LocalTransportMessageSendingRepository {
    #[tracing::instrument(name = "send", skip_all)]
    async fn send(
        &self,
        message: Message,
    ) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors> {
        let email = build_lettre_message(&message)?;

        let sent: Result<(), String> = match &self.transport {
            LocalTransportKind::Smtp(t) => {
                t.send(&email).map(|_| ()).map_err(|e| e.to_string())
            }
            LocalTransportKind::File(t) => {
                t.send(&email).map(|_| ()).map_err(|e| e.to_string())
            }
            LocalTransportKind::Stub(t) => {
                t.send(&email).map(|_| ()).map_err(|e| e.to_string())
            }
        };

        match sent {
            Ok(_) => {
                // The stub never delivers -- render the message as a
                // human-friendly block on stdout so a developer can read a
                // magic-link code straight from the terminal. Deliberately
                // bypasses `tracing` (log level/format, SigNoz export): this
                // is a local-dev affordance, not observability signal.
                if matches!(self.transport, LocalTransportKind::Stub(_)) {
                    println!("{}", render_stub_email_for_terminal(&message));
                }

                Ok(CreateResponseKind::Created(None))
            }
            Err(err) => {
                creation_err(format!("Could not send email: {err}")).as_error()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use myc_core::domain::dtos::{email::Email, message::FromEmail};

    #[test]
    fn smtp_takes_precedence_when_configured() {
        let smtp = SmtpTransport::builder_dangerous("localhost").build();

        let selected =
            select_local_transport(Some(smtp), Some(PathBuf::from("/tmp")));

        assert!(matches!(selected, LocalTransportKind::Smtp(_)));
    }

    #[test]
    fn file_is_selected_when_smtp_absent_and_dir_configured() {
        let selected =
            select_local_transport(None, Some(PathBuf::from("/tmp")));

        assert!(matches!(selected, LocalTransportKind::File(_)));
    }

    #[test]
    fn stub_is_selected_when_neither_smtp_nor_dir_configured() {
        let selected = select_local_transport(None, None);

        assert!(matches!(selected, LocalTransportKind::Stub(_)));
    }

    fn sample_message(body: &str) -> Message {
        Message {
            from: FromEmail::Email(
                Email::from_string("noreply@mycelium.com".to_string()).unwrap(),
            ),
            to: Email::from_string("user@mycelium.com".to_string()).unwrap(),
            cc: None,
            subject: "Your magic link".to_string(),
            body: body.to_string(),
        }
    }

    // Terminal rendering of the stub email is covered by
    // `render_stub_email_for_terminal`'s own unit tests; here we only assert
    // the stub send path succeeds (it prints to stdout, not `tracing`).
    #[tokio::test]
    async fn stub_transport_send_succeeds() {
        let repo = LocalTransportMessageSendingRepository {
            transport: LocalTransportKind::Stub(StubTransport::new_ok()),
        };

        let message =
            sample_message("Click https://mycelium.com/magic-link/abc123");

        repo.send(message).await.unwrap();
    }

    #[tokio::test]
    async fn file_transport_writes_a_parseable_eml() {
        let dir = std::env::temp_dir().join(format!(
            "myc_notifier_local_transport_{}_{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let repo = LocalTransportMessageSendingRepository {
            transport: LocalTransportKind::File(FileTransport::new(&dir)),
        };

        let message = sample_message("Hello from the file transport");

        repo.send(message).await.unwrap();

        let entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(entries.len(), 1);

        let content = std::fs::read_to_string(entries[0].path()).unwrap();
        assert!(content.contains("Subject: Your magic link"));
        assert!(content.contains("Hello from the file transport"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
