use crate::functions::{connect_sql_module, try_to_resolve_database_url};

use clap::Parser;
use myc_core::domain::{
    dtos::federation::{FederationPolicy, NewFederationPeer},
    entities::{FederationPeerRead, FederationPeerWrite},
};
use myc_diesel::repositories::SqlAppModule;
use mycelium_base::{
    entities::{
        CreateResponseKind, DeletionResponseKind, FetchManyResponseKind,
        FetchResponseKind,
    },
    utils::errors::{execution_err, MappedErrors},
};
use shaku::HasComponent;
use std::process::exit;
use uuid::Uuid;

#[derive(Parser, Debug)]
pub(crate) struct PeerArguments {
    #[clap(subcommand)]
    pub cmd: PeerCommands,
}

#[derive(Parser, Debug, PartialEq)]
pub(crate) enum PeerCommands {
    /// Declare a peer; the gateway handshakes with it on its next health
    /// round. Its policy starts denying everything (FED-10).
    Add(AddPeerArguments),

    /// Print every declared peer as JSON
    List,

    /// Remove a peer with its stored keys and nonces
    Remove {
        /// The `instanceId` the peer was declared with
        #[clap(long)]
        instance_id: Uuid,
    },
}

#[derive(Parser, Debug, PartialEq)]
pub(crate) struct AddPeerArguments {
    /// Canonical base URL of the peer, as it announces it
    #[clap(long)]
    pub base_url: String,

    /// The `instanceId` the peer must announce; a mismatch marks it untrusted
    #[clap(long)]
    pub instance_id: Uuid,

    #[clap(long)]
    pub display_name: Option<String>,

    /// Assertion kinds this instance accepts from the peer (repeatable)
    #[clap(long = "consume")]
    pub consume_kinds: Vec<String>,

    /// Assertion kinds this instance serves to the peer (repeatable)
    #[clap(long = "serve")]
    pub serve_kinds: Vec<String>,
}

impl AddPeerArguments {
    fn into_new_peer(self) -> NewFederationPeer {
        NewFederationPeer {
            instance_id: self.instance_id,
            base_url: self.base_url,
            display_name: self.display_name,
            consume_kinds: self.consume_kinds,
            serve_kinds: self.serve_kinds,
            policy: FederationPolicy::default(),
        }
    }
}

/// Run a peer command; exit 1 with the reason on any error
pub(crate) async fn peer_cmd(cmd: PeerCommands) {
    let module = sql_module();
    let outcome = match cmd {
        PeerCommands::Add(args) => add_peer(&module, args).await,
        PeerCommands::List => list_peers(&module).await,
        PeerCommands::Remove { instance_id } => {
            remove_peer(&module, instance_id).await
        }
    };

    if let Err(err) = outcome {
        tracing::error!("federation peer command failed: {err}");
        exit(1);
    }
}

fn sql_module() -> SqlAppModule {
    connect_sql_module(&try_to_resolve_database_url())
}

fn print_json<T: serde::Serialize>(value: &T) -> Result<(), MappedErrors> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| execution_err(e.to_string()))?;
    println!("{json}");
    Ok(())
}

async fn add_peer(
    module: &SqlAppModule,
    args: AddPeerArguments,
) -> Result<(), MappedErrors> {
    let repo: &dyn FederationPeerWrite = module.resolve_ref();

    match repo.declare(args.into_new_peer(), None).await? {
        CreateResponseKind::Created(peer) => print_json(&peer),
        CreateResponseKind::NotCreated(_, reason) => {
            execution_err(reason).as_error()
        }
    }
}

async fn list_peers(module: &SqlAppModule) -> Result<(), MappedErrors> {
    let repo: &dyn FederationPeerRead = module.resolve_ref();

    match repo.list().await? {
        FetchManyResponseKind::Found(peers) => print_json(&peers),
        _ => print_json(&Vec::<()>::new()),
    }
}

async fn remove_peer(
    module: &SqlAppModule,
    instance_id: Uuid,
) -> Result<(), MappedErrors> {
    let read: &dyn FederationPeerRead = module.resolve_ref();
    let FetchResponseKind::Found(peer) =
        read.get_by_instance_id(instance_id).await?
    else {
        return execution_err(format!(
            "no peer with instance id {instance_id}"
        ))
        .as_error();
    };
    let write: &dyn FederationPeerWrite = module.resolve_ref();

    match write.remove(peer.id).await? {
        DeletionResponseKind::Deleted => Ok(()),
        DeletionResponseKind::NotDeleted(_, reason) => {
            execution_err(reason).as_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cmds::federation, Cli};

    fn parse(args: &[&str]) -> PeerCommands {
        let mut argv = vec!["myc-cli", "federation", "peer"];
        argv.extend_from_slice(args);

        let Cli::Federation(federation::Arguments {
            cmd: federation::Commands::Peer(PeerArguments { cmd }),
        }) = Cli::try_parse_from(argv).unwrap()
        else {
            panic!("not a federation peer command");
        };

        cmd
    }

    #[test]
    fn add_takes_the_url_the_instance_id_and_repeated_kinds() {
        let instance_id = Uuid::new_v4();
        let id = instance_id.to_string();

        let PeerCommands::Add(args) = parse(&[
            "add",
            "--base-url",
            "https://myc-02.example.com",
            "--instance-id",
            &id,
            "--consume",
            "profile.v1",
            "--serve",
            "profile.v1",
            "--serve",
            "tenant.v1",
        ]) else {
            panic!("not add");
        };
        let peer = args.into_new_peer();

        assert_eq!(peer.instance_id, instance_id);
        assert_eq!(peer.base_url, "https://myc-02.example.com");
        assert_eq!(peer.consume_kinds, ["profile.v1"]);
        assert_eq!(peer.serve_kinds, ["profile.v1", "tenant.v1"]);
        assert_eq!(peer.policy, FederationPolicy::default());
    }

    #[test]
    fn add_requires_the_instance_id() {
        let argv = [
            "myc-cli",
            "federation",
            "peer",
            "add",
            "--base-url",
            "https://myc-02.example.com",
        ];

        assert!(Cli::try_parse_from(argv).is_err());
    }

    #[test]
    fn list_and_remove_parse() {
        let instance_id = Uuid::new_v4();
        let id = instance_id.to_string();

        assert_eq!(parse(&["list"]), PeerCommands::List);
        assert_eq!(
            parse(&["remove", "--instance-id", &id]),
            PeerCommands::Remove { instance_id }
        );
    }
}
