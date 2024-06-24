#[macro_use]
extern crate serde_derive;
mod controller;
use az_group_crd;
use az_group_manager_crd;
use clap::{Parser, Subcommand, ValueEnum};
use controller::reconciler;
use std::error::Error;

use tracing::{info, Level};

#[derive(Parser, Debug)]
#[command(
    name = "az-group-fetcher",
    about,
    version,
    after_help = "Author: Patrick Kerwood <patrick@kerwood.dk>",
    disable_help_subcommand = true
)]
struct Opt {
    #[command(subcommand)]
    pub command: SubCommand,

    #[arg(long, env, help = "Logs will be output as JSON.", global = true)]
    structured_logs: bool,

    #[arg(
        short = 'l',
        long,
        env,
        default_value = "info",
        help = "Log Level.",
        global = true
    )]
    log_level: LogLevel,
}

#[derive(Subcommand, Debug)]
enum SubCommand {
    /// Start the service.
    Serve {
        #[arg(short = 't', long, env, help = "Azure Tenant ID.")]
        azure_tenant_id: String,

        #[arg(short = 'i', long, env, help = "Service Principal Client ID.")]
        azure_client_id: String,

        #[arg(short = 's', long, env, help = "Service Principal Client Secret.")]
        azure_client_secret: String,

        #[arg(
            short = 'b',
            long,
            env,
            default_value = "300",
            help = "Seconds between each reconciliation."
        )]
        reconcile_time: String,

        #[arg(
            short = 'r',
            long,
            env,
            default_value = "10",
            help = "Seconds between each retry if reconciliation fails."
        )]
        retry_time: String,
    },

    /// Print the Custom Resource Definition for AzureGroup.
    PrintCrd {},
}

#[derive(Debug, Clone, ValueEnum)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for Level {
    fn from(log_level: LogLevel) -> Self {
        match log_level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::parse();

    let log_level: Level = opt.log_level.into();
    let subscriber_builder = tracing_subscriber::fmt().with_max_level(log_level);

    match opt.structured_logs {
        true => subscriber_builder.json().init(),
        false => subscriber_builder.init(),
    };

    match opt.command {
        SubCommand::Serve {
            azure_client_id,
            azure_client_secret,
            azure_tenant_id,
            reconcile_time,
            retry_time,
        } => {
            info!("Running application!");
            _ = reconciler::run(reconciler::Args {
                azure_tenant_id,
                azure_client_id,
                azure_client_secret,
                reconcile_time: reconcile_time.parse()?,
                retry_time: retry_time.parse()?,
            })
            .await;
        }
        SubCommand::PrintCrd {} => {
            let manager_crd = az_group_manager_crd::print_crd().unwrap();
            let group_crd = az_group_crd::print_crd().unwrap();
            println!("{}\n---\n{}", manager_crd, group_crd);
        }
    }
    Ok(())
}
