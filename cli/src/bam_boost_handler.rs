use anyhow::anyhow;
use borsh::BorshDeserialize;
use jito_bam_boost_client::{accounts::ClaimStatus, instructions::ClaimBuilder};
use jito_bam_boost_merkle_tree::bam_boost_merkle_tree::BamBoostMerkleTree;
use solana_keypair::Signer;
use solana_pubkey::Pubkey;
use solana_rpc_client::rpc_client::RpcClient;
use solana_transaction::{Instruction, Signers, Transaction};
use spl_associated_token_account_interface::{
    address::get_associated_token_address_with_program_id,
    instruction::create_associated_token_account_idempotent,
};

use crate::{
    bam_boost::{BamBoostCommands, ClaimStatusActions, MerkleDistributorActions, NetworkArg},
    cli_config::CliConfig,
    JITOSOL_MINT,
};

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
            BamBoostCommands::ClaimStatus {
                action: ClaimStatusActions::Get { epoch, claimant },
            } => self.get_claim_status(epoch, claimant),
        }
    }

    fn merkle_distributor_address(&self, jitosol_mint: Pubkey, epoch: u64) -> Pubkey {
        Pubkey::find_program_address(
            &[
                b"merkle_distributor",
                jitosol_mint.to_bytes().as_slice(),
                epoch.to_le_bytes().as_slice(),
            ],
            &self.bam_boost_program_id,
        )
        .0
    }

    fn claim_status_address(&self, claimant: Pubkey, distributor_pda: Pubkey) -> Pubkey {
        Pubkey::find_program_address(
            &[
                b"claim_status",
                claimant.to_bytes().as_slice(),
                distributor_pda.to_bytes().as_slice(),
            ],
            &self.bam_boost_program_id,
        )
        .0
    }

    async fn claim(&self, cluster: &str, epoch: u64) -> anyhow::Result<()> {
        let rpc_client = self.get_rpc_client();
        let signer = self
            .cli_config
            .signer
            .clone()
            .ok_or_else(|| anyhow::anyhow!("signer is required"))?;

        let distributor_pda = self.merkle_distributor_address(JITOSOL_MINT, epoch);
        let distributor_token_address = get_associated_token_address_with_program_id(
            &Pubkey::new_from_array(distributor_pda.to_bytes()),
            &JITOSOL_MINT,
            &spl_token_interface::id(),
        );

        let claim_status_pda = self.claim_status_address(signer.pubkey(), distributor_pda);
        let claimant_token_address = get_associated_token_address_with_program_id(
            &signer.pubkey(),
            &JITOSOL_MINT,
            &spl_token_interface::id(),
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

        let node = merkle_tree.get_node(&signer.pubkey());

        let claim_status_pda = Pubkey::new_from_array(claim_status_pda.to_bytes());

        if rpc_client.get_account(&claim_status_pda).is_ok() {
            return Err(anyhow!("Claim status account already exists — subsidy for this epoch has already been claimed."));
        }

        let mut ix_builder = ClaimBuilder::new();
        ix_builder
            .distributor(Pubkey::new_from_array(distributor_pda.to_bytes()))
            .claim_status(claim_status_pda)
            .from(distributor_token_address)
            .to(claimant_token_address)
            .claimant(signer.pubkey())
            .token_program(spl_token_interface::id())
            .amount(node.amount)
            .proof(node.proof.unwrap());
        let mut ix = ix_builder.instruction();
        ix.program_id = self.bam_boost_program_id;

        log::info!("Claiming parameters: {ix_builder:?}");

        self.process_transaction(
            &[
                create_associated_token_account_idempotent(
                    &signer.pubkey(),
                    &signer.pubkey(),
                    &JITOSOL_MINT,
                    &spl_token_interface::id(),
                ),
                ix,
            ],
            &signer.pubkey(),
            &[signer],
        )?;

        if !self.print_tx {
            let claim_status_acc = self
                .get_account::<ClaimStatus>(&Pubkey::new_from_array(claim_status_pda.to_bytes()))?;
            log::info!("ClaimStatus: {claim_status_acc:?}");
        }

        Ok(())
    }

    fn get_claim_status(&self, epoch: u64, claimant: Pubkey) -> anyhow::Result<()> {
        let distributor_pda = self.merkle_distributor_address(JITOSOL_MINT, epoch);

        let claim_status_pda = self.claim_status_address(claimant, distributor_pda);

        println!("ClaimStatus PDA: {claim_status_pda}");

        let account =
            self.get_account::<ClaimStatus>(&Pubkey::new_from_array(claim_status_pda.to_bytes()))?;

        println!("{}", serde_json::to_string_pretty(&account)?);

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
