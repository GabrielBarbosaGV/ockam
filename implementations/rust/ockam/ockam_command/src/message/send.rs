use clap::Args;
use miette::{Context as _, IntoDiagnostic};

use core::time::Duration;
use ockam::{Context, TcpTransport};
use ockam_api::nodes::models::secure_channel::CredentialExchangeMode;
use ockam_api::nodes::service::message::SendMessage;
use ockam_core::api::{Request, RequestBuilder};
use ockam_multiaddr::MultiAddr;

use crate::identity::{get_identity_name, initialize_identity_if_default};
use crate::node::util::{delete_embedded_node, start_embedded_node_with_vault_and_identity};
use crate::util::api::{CloudOpts, TrustContextOpts};
use crate::util::{clean_nodes_multiaddr, extract_address_value, node_rpc, RpcBuilder};

use crate::{docs, CommandGlobalOpts};

const LONG_ABOUT: &str = include_str!("./static/send/long_about.txt");
const AFTER_LONG_HELP: &str = include_str!("./static/send/after_long_help.txt");

/// Send a message to an Ockam node
#[derive(Clone, Debug, Args)]
#[command(
arg_required_else_help = true,
long_about = docs::about(LONG_ABOUT),
after_long_help = docs::after_help(AFTER_LONG_HELP)
)]
pub struct SendCommand {
    /// The node to send messages from
    #[arg(short, long, value_name = "NODE")]
    from: Option<String>,

    /// The route to send the message to
    #[arg(short, long, value_name = "ROUTE")]
    pub to: MultiAddr,

    /// Flag to indicate that the message is hex encoded
    #[arg(long)]
    pub hex: bool,

    /// Override default timeout (in seconds)
    #[arg(long, value_name = "TIMEOUT", default_value = "10")]
    pub timeout: u64,

    pub message: String,

    #[command(flatten)]
    cloud_opts: CloudOpts,

    #[command(flatten)]
    pub trust_context_opts: TrustContextOpts,
}

impl SendCommand {
    pub fn run(self, opts: CommandGlobalOpts) {
        initialize_identity_if_default(&opts, &self.cloud_opts.identity);
        node_rpc(rpc, (opts, self))
    }
}

async fn rpc(
    mut ctx: Context,
    (opts, cmd): (CommandGlobalOpts, SendCommand),
) -> miette::Result<()> {
    async fn go(
        ctx: &mut Context,
        opts: CommandGlobalOpts,
        cmd: SendCommand,
    ) -> miette::Result<()> {
        // Setup environment depending on whether we are sending the message from an embedded node or a background node
        let (api_node, tcp) = if let Some(node) = &cmd.from {
            let api_node = extract_address_value(node)?;
            let tcp = TcpTransport::create(ctx).await.into_diagnostic()?;
            (api_node, Some(tcp))
        } else {
            let identity = get_identity_name(&opts.state, &cmd.cloud_opts.identity);
            let api_node = start_embedded_node_with_vault_and_identity(
                ctx,
                &opts,
                None,
                Some(identity),
                Some(&cmd.trust_context_opts),
            )
            .await?;
            (api_node, None)
        };

        // Process `--to` Multiaddr
        let (to, meta) =
            clean_nodes_multiaddr(&cmd.to, &opts.state).context("Argument '--to' is invalid")?;

        // Replace `/project/<name>` occurrences with their respective secure channel addresses
        let projects_sc = crate::project::util::get_projects_secure_channels_from_config_lookup(
            ctx,
            &opts,
            &meta,
            &api_node,
            tcp.as_ref(),
            CredentialExchangeMode::Oneway,
        )
        .await?;
        let to = crate::project::util::clean_projects_multiaddr(to, projects_sc)?;
        // Send request
        let mut rpc = RpcBuilder::new(ctx, &opts, &api_node)
            .tcp(tcp.as_ref())?
            .build();

        let msg_bytes = if cmd.hex {
            hex::decode(cmd.message)
                .into_diagnostic()
                .context("The message is not a valid hex string")?
        } else {
            cmd.message.as_bytes().to_vec()
        };

        rpc.request_with_timeout(
            req(&to, msg_bytes.as_slice()),
            Duration::from_secs(cmd.timeout),
        )
        .await?;
        let res = {
            let res = rpc.parse_response::<Vec<u8>>()?;
            if cmd.hex {
                hex::encode(res)
            } else {
                String::from_utf8(res)
                    .into_diagnostic()
                    .context("Received content is not a valid utf8 string")?
            }
        };

        // only delete node in case 'from' is empty and embedded node was started before
        if cmd.from.is_none() {
            delete_embedded_node(&opts, rpc.node_name()).await;
        }

        opts.terminal.stdout().plain(&res).write_line()?;

        Ok(())
    }
    go(&mut ctx, opts, cmd).await
}

pub(crate) fn req<'a>(to: &'a MultiAddr, message: &'a [u8]) -> RequestBuilder<'a, SendMessage<'a>> {
    Request::post("v0/message").body(SendMessage::new(to, message))
}
