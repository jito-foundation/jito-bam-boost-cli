use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::bam_boost::BamBoostCommands;

#[derive(Parser)]
#[command(author, version, about = "A CLI for managing BAM Boost operations", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<ProgramCommand>,

    #[arg(long, global = true, help = "Path to the configuration file")]
    pub config_file: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        default_value = "https://api.mainnet-beta.solana.com",
        help = "RPC URL to use"
    )]
    pub rpc_url: Option<String>,

    #[arg(long, global = true, help = "Commitment level")]
    pub commitment: Option<String>,

    #[arg(long, global = true, help = "BAM Boost program ID")]
    pub jito_bam_boost_program_id: Option<String>,

    #[arg(long, global = true, help = "Filepath or URL to a keypair")]
    pub signer: Option<String>,

    #[arg(long, global = true, help = "Verbose mode")]
    pub verbose: bool,

    #[arg(
        long,
        global = true,
        default_value = "false",
        help = "This will print out the raw TX instead of running it"
    )]
    pub print_tx: bool,

    #[arg(
        long,
        global = true,
        default_value = "false",
        help = "This will print out account information in JSON format"
    )]
    pub print_json: bool,

    #[arg(
        long,
        global = true,
        default_value = "false",
        help = "This will print out account information in JSON format with reserved space"
    )]
    pub print_json_with_reserves: bool,

    #[arg(long, global = true, hide = true)]
    pub markdown_help: bool,
}

#[derive(Subcommand)]
pub enum ProgramCommand {
    /// BAM Boost program commands
    BamBoost {
        #[command(subcommand)]
        action: BamBoostCommands,
    },
}
