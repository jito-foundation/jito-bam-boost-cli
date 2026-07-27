use std::{str::FromStr, sync::Arc};

use clap::Parser;
use env_logger::Env;
use jito_bam_boost_cli::{
    bam_boost_handler::BamBoostCliHandler,
    cli_args::{Cli, ProgramCommand},
    cli_config::CliConfig,
};
use jito_bam_boost_client::programs::JITO_BAM_BOOST_PROGRAM_ID;
use solana_commitment_config::CommitmentConfig;
use solana_keypair::read_keypair_file;
use solana_pubkey::Pubkey;

pub fn get_cli_config(args: &Cli) -> Result<CliConfig, anyhow::Error> {
    let signer = match &args.signer {
        Some(path) => {
            let keypair = read_keypair_file(path)
                .map_err(|e| anyhow::anyhow!("Failed to read keypair: {}", e))?;
            Some(Arc::new(keypair))
        }
        _ => None,
    };

    let cli_config = CliConfig {
        rpc_url: args
            .rpc_url
            .clone()
            .ok_or_else(|| anyhow::anyhow!("rpc_url is required"))?,
        commitment: CommitmentConfig::from_str(
            args.commitment
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("commitment is required"))?,
        )?,
        signer,
    };

    Ok(cli_config)
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args: Cli = Cli::parse();

    let bam_boost_program_id =
        if let Some(jito_bam_boost_program_id) = &args.jito_bam_boost_program_id {
            Pubkey::from_str(jito_bam_boost_program_id)?
        } else {
            JITO_BAM_BOOST_PROGRAM_ID
        };

    // The TUI owns the alternate screen; env_logger writing to stderr would
    // corrupt it, and the TUI has relaxed config requirements (no
    // --commitment needed), so it is handled before logger init and
    // get_cli_config.
    if let Some(ProgramCommand::Tui { network }) = &args.command {
        let commitment = match &args.commitment {
            Some(c) => CommitmentConfig::from_str(c)?,
            None => CommitmentConfig::confirmed(),
        };
        let rpc_url = args
            .rpc_url
            .clone()
            .unwrap_or_else(|| "https://api.mainnet-beta.solana.com".to_string());
        return jito_bam_boost_cli::tui::run(
            network.clone(),
            rpc_url,
            commitment,
            args.signer.clone(),
            bam_boost_program_id,
        )
        .await;
    }

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let cli_config = get_cli_config(&args)?;

    match args.command.expect("Command not found") {
        ProgramCommand::BamBoost { action } => {
            BamBoostCliHandler::new(
                cli_config,
                bam_boost_program_id,
                args.print_tx,
                args.print_json,
                args.print_json_with_reserves,
            )
            .handle(action)
            .await?;
        }
        ProgramCommand::Tui { .. } => unreachable!("handled above"),
    }

    Ok(())
}
