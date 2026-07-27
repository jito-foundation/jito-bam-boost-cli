use anyhow::anyhow;
use borsh::BorshDeserialize;
use jito_bam_boost_client::accounts::ClaimStatus;
use jito_bam_boost_merkle_tree::bam_boost_merkle_tree::BamBoostMerkleTree;
use solana_keypair::Signer;
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;
use solana_transaction::{Instruction, Signers, Transaction};

use crate::{
    bam_boost::{BamBoostCommands, ClaimStatusActions, MerkleDistributorActions, NetworkArg},
    cli_config::CliConfig,
    pda, JITOSOL_MINT,
};

// `format_jitosol` lives in `scanner.rs` (single implementation); re-exported here so
// existing `bam_boost_handler::format_jitosol` imports keep compiling.
pub use crate::scanner::format_jitosol;

/// Formats a message about skipped expired epochs.
fn format_expired_message(count: usize, total_amount: u64) -> String {
    format!(
        "Skipping {} expired epoch(s) ({} JitoSOL no longer claimable)",
        count,
        format_jitosol(total_amount)
    )
}

pub fn default_cache_dir() -> std::path::PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("jito-bam-boost")
}

#[allow(dead_code)]
pub struct BamBoostCliHandler {
    /// The configuration of CLI
    cli_config: CliConfig,

    /// The Pubkey of the Jito BAM Boost Program
    bam_boost_program_id: Pubkey,

    /// This will print out the raw TX instead of running it
    print_tx: bool,

    /// This will print out the account information in JSON format
    print_json: bool,

    /// This will print out the account information in JSON format with reserved space
    print_json_with_reserves: bool,
}

impl BamBoostCliHandler {
    pub const fn new(
        cli_config: CliConfig,
        bam_boost_program_id: Pubkey,
        print_tx: bool,
        print_json: bool,
        print_json_with_reserves: bool,
    ) -> Self {
        Self {
            cli_config,
            bam_boost_program_id,
            print_tx,
            print_json,
            print_json_with_reserves,
        }
    }

    pub async fn handle(&self, action: BamBoostCommands) -> anyhow::Result<()> {
        match action {
            BamBoostCommands::MerkleDistributor {
                action: MerkleDistributorActions::Claim { network, epoch },
            } => {
                let network = match network {
                    NetworkArg::Mainnet => "mainnet",
                    NetworkArg::Testnet => "testnet",
                };

                self.claim(network, epoch).await
            }
            BamBoostCommands::MerkleDistributor {
                action: MerkleDistributorActions::Status { network, claimant },
            } => {
                let network = match network {
                    NetworkArg::Mainnet => "mainnet",
                    NetworkArg::Testnet => "testnet",
                };
                self.status(network, claimant).await
            }
            BamBoostCommands::MerkleDistributor {
                action: MerkleDistributorActions::ClaimAll { network, yes },
            } => {
                let network = match network {
                    NetworkArg::Mainnet => "mainnet",
                    NetworkArg::Testnet => "testnet",
                };
                self.claim_all(network, yes).await
            }
            BamBoostCommands::ClaimStatus {
                action: ClaimStatusActions::Get { epoch, claimant },
            } => self.get_claim_status(epoch, claimant),
        }
    }

    async fn claim(&self, cluster: &str, epoch: u64) -> anyhow::Result<()> {
        let rpc_client = self.get_rpc_client();
        let signer = self
            .cli_config
            .signer
            .clone()
            .ok_or_else(|| anyhow::anyhow!("signer is required"))?;

        let distributor_pda =
            pda::merkle_distributor_address(&self.bam_boost_program_id, &JITOSOL_MINT, epoch);
        let claim_status_pda = pda::claim_status_address(
            &self.bam_boost_program_id,
            &signer.pubkey(),
            &distributor_pda,
        );

        let url = format!(
            "https://storage.googleapis.com/jito-bam-boost/{cluster}/{epoch}/merkle_tree.json",
        );

        log::info!("Fetching merkle tree from: {}", url);

        // Download the merkle tree JSON from GCS
        let response = match reqwest::get(&url).await {
            Ok(resp) => resp,
            Err(e) => {
                log::error!("Failed to fetch merkle tree: {}", e);
                return Err(anyhow!("Failed to fetch merkle tree: {e}"));
            }
        };

        let response_json = match response.json().await {
            Ok(json) => json,
            Err(e) => {
                log::error!("Failed to parse merkle tree JSON response: {e}");
                return Err(anyhow!("Failed to parse merkle tree JSON response: {e}"));
            }
        };

        // Parse the merkle tree JSON (amounts are already in lamports, no conversion needed)
        let merkle_tree: BamBoostMerkleTree =
            match BamBoostMerkleTree::new_from_entries(response_json) {
                Ok(tree) => tree,
                Err(e) => {
                    log::error!("Failed to parse merkle tree: {e}");
                    return Err(anyhow!("Failed to parse merkle tree: {e}"));
                }
            };

        if rpc_client.get_account(&claim_status_pda).is_ok() {
            return Err(anyhow!("Claim status account already exists — subsidy for this epoch has already been claimed."));
        }

        let node = merkle_tree.get_node(&signer.pubkey());
        let proof = node
            .proof
            .clone()
            .ok_or_else(|| anyhow!("merkle proof missing for claimant"))?;
        let ixs = crate::batch_claim::build_claim_ixs(
            &self.bam_boost_program_id,
            &signer.pubkey(),
            epoch,
            node.amount,
            proof,
        );

        log::info!("Claiming epoch {epoch} for {}", signer.pubkey());

        self.process_transaction(&ixs, &signer.pubkey(), &[signer])?;

        if !self.print_tx {
            let claim_status_acc = self.get_account::<ClaimStatus>(&claim_status_pda)?;
            log::info!("ClaimStatus: {claim_status_acc:?}");
        }

        Ok(())
    }

    async fn status(&self, network: &str, claimant: Pubkey) -> anyhow::Result<()> {
        let scanner = crate::scanner::Scanner::new(default_cache_dir());
        let rpc_client = self.get_rpc_client();
        let statuses = scanner
            .scan(network, &claimant, &rpc_client, &self.bam_boost_program_id)
            .await?;

        if self.print_json {
            println!("{}", serde_json::to_string_pretty(&statuses)?);
            return Ok(());
        }

        println!("{:>8}  {:>18}  Status", "Epoch", "Amount (JitoSOL)");
        for s in &statuses {
            let (amount, state) = match (s.amount, s.claimed, s.expired) {
                (Some(a), true, _) => (format_jitosol(a), "claimed"),
                (Some(a), false, true) => (format_jitosol(a), "expired"),
                (Some(a), false, false) => (format_jitosol(a), "unclaimed"),
                (None, _, _) => ("-".to_string(), "not eligible"),
            };
            println!("{:>8}  {:>18}  {}", s.epoch, amount, state);
        }
        println!(
            "\n{}",
            crate::scanner::Stats::from(statuses.as_slice()).format()
        );
        Ok(())
    }

    fn get_claim_status(&self, epoch: u64, claimant: Pubkey) -> anyhow::Result<()> {
        let distributor_pda =
            pda::merkle_distributor_address(&self.bam_boost_program_id, &JITOSOL_MINT, epoch);

        let claim_status_pda =
            pda::claim_status_address(&self.bam_boost_program_id, &claimant, &distributor_pda);

        println!("ClaimStatus PDA: {claim_status_pda}");

        let account =
            self.get_account::<ClaimStatus>(&Pubkey::new_from_array(claim_status_pda.to_bytes()))?;

        println!("{}", serde_json::to_string_pretty(&account)?);

        Ok(())
    }

    async fn claim_all(&self, network: &str, yes: bool) -> anyhow::Result<()> {
        if self.print_tx {
            anyhow::bail!(
                "--print-tx is not supported by claim-all; use `status` to preview unclaimed epochs"
            );
        }

        let signer = self
            .cli_config
            .signer
            .clone()
            .ok_or_else(|| anyhow::anyhow!("signer is required"))?;
        let claimant = signer.pubkey();

        let scanner = crate::scanner::Scanner::new(default_cache_dir());
        let rpc_client = self.get_rpc_client();
        let statuses = scanner
            .scan(network, &claimant, &rpc_client, &self.bam_boost_program_id)
            .await?;

        // Print stats after scan
        println!("{}", crate::scanner::Stats::from(&statuses[..]).format());

        // Print message about skipped expired epochs if any exist
        let expired: Vec<_> = statuses.iter().filter(|s| s.expired).collect();
        if !expired.is_empty() {
            let expired_amount: u64 = expired.iter().filter_map(|s| s.amount).sum();
            println!("{}", format_expired_message(expired.len(), expired_amount));
        }

        let unclaimed: Vec<_> = statuses.iter().filter(|s| s.is_claimable()).collect();
        if unclaimed.is_empty() {
            println!("Nothing to claim: no unclaimed epochs for {claimant}");
            return Ok(());
        }

        let total: u64 = unclaimed.iter().filter_map(|s| s.amount).sum();
        println!("Unclaimed epochs for {claimant}:");
        for s in &unclaimed {
            println!(
                "  epoch {:>6}: {} JitoSOL",
                s.epoch,
                format_jitosol(s.amount.unwrap_or(0))
            );
        }
        println!(
            "Total: {} JitoSOL across {} epoch(s)",
            format_jitosol(total),
            unclaimed.len()
        );

        if !yes {
            print!("Proceed with claiming? [y/N] ");
            use std::io::Write as _;
            std::io::stdout().flush()?;
            let mut answer = String::new();
            std::io::stdin().read_line(&mut answer)?;
            if !matches!(answer.trim(), "y" | "Y" | "yes") {
                println!("Aborted.");
                return Ok(());
            }
        }

        let epochs: Vec<u64> = unclaimed.iter().map(|s| s.epoch).collect();
        let results = crate::batch_claim::claim_epochs(
            &scanner,
            &self.cli_config,
            &self.bam_boost_program_id,
            network,
            &epochs,
            &mut |event| match &event.state {
                crate::batch_claim::ClaimState::Started => {
                    println!("epoch {}: claiming...", event.epoch)
                }
                crate::batch_claim::ClaimState::Success(sig) => {
                    println!("epoch {}: OK  {sig}", event.epoch)
                }
                crate::batch_claim::ClaimState::Failed(e) => {
                    println!("epoch {}: FAILED  {e}", event.epoch)
                }
                crate::batch_claim::ClaimState::Skipped(r) => {
                    println!("epoch {}: skipped  {r}", event.epoch)
                }
            },
        )
        .await?;

        let ok = results
            .iter()
            .filter(|r| matches!(r.state, crate::batch_claim::ClaimState::Success(_)))
            .count();
        let failed = results
            .iter()
            .filter(|r| matches!(r.state, crate::batch_claim::ClaimState::Failed(_)))
            .count();
        println!(
            "\nDone: {ok} claimed, {failed} failed, {} other",
            results.len() - ok - failed
        );
        Ok(())
    }

    /// Creates a new RPC client using the configuration from the CLI handler.
    ///
    /// This method constructs an RPC client with the URL and commitment level specified in the
    /// CLI configuration. The client can be used to communicate with a Solana node for
    /// submitting transactions, querying account data, and other RPC operations.
    fn get_rpc_client(&self) -> RpcClient {
        RpcClient::new_with_commitment(self.cli_config.rpc_url.clone(), self.cli_config.commitment)
    }

    /// Fetches and deserializes an account
    ///
    /// This method retrieves account data using the configured RPC client,
    /// then deserializes it into the specified account type using Borsh deserialization.
    fn get_account<T: BorshDeserialize>(&self, account_pubkey: &Pubkey) -> anyhow::Result<T> {
        let rpc_client = self.get_rpc_client();

        let account = rpc_client.get_account(account_pubkey)?;
        let account = T::deserialize(&mut account.data.as_slice())?;

        Ok(account)
    }

    /// Processes a transaction by either printing it as Base58 or sending it.
    ///
    /// This method handles the logic for processing a set of instructions as a transaction.
    /// If `print_tx` is enabled in the CLI handler (helpful for running commands in Squads), it will print the transaction in Base58 format
    /// without sending it. Otherwise, it will submit and confirm the transaction.
    fn process_transaction<T>(
        &self,
        ixs: &[Instruction],
        payer: &Pubkey,
        signers: &T,
    ) -> anyhow::Result<()>
    where
        T: Signers + ?Sized,
    {
        let rpc_client = self.get_rpc_client();

        let blockhash = rpc_client.get_latest_blockhash()?;
        let tx = Transaction::new_signed_with_payer(ixs, Some(payer), signers, blockhash);
        let result = rpc_client.send_and_confirm_transaction(&tx)?;

        log::info!("Transaction confirmed: {:?}", result);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_jitosol_amounts() {
        assert_eq!(format_jitosol(0), "0.000000000");
        assert_eq!(format_jitosol(1_234), "0.000001234");
        assert_eq!(format_jitosol(1_500_000_000), "1.500000000");
    }

    #[test]
    fn formats_expired_message() {
        assert_eq!(
            format_expired_message(1, 500_000_000),
            "Skipping 1 expired epoch(s) (0.500000000 JitoSOL no longer claimable)"
        );
        assert_eq!(
            format_expired_message(3, 3_500_000_000),
            "Skipping 3 expired epoch(s) (3.500000000 JitoSOL no longer claimable)"
        );
    }

    #[tokio::test]
    async fn claim_all_rejects_print_tx() {
        let cli_config = CliConfig {
            rpc_url: "http://localhost:1".to_string(),
            commitment: solana_commitment_config::CommitmentConfig::confirmed(),
            signer: None,
        };
        let handler = BamBoostCliHandler::new(cli_config, Pubkey::new_unique(), true, false, false);

        let err = handler
            .claim_all("mainnet", true)
            .await
            .expect_err("claim-all must reject --print-tx");
        assert!(err
            .to_string()
            .contains("--print-tx is not supported by claim-all"));
    }
}
