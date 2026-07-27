# Jito BAM Boost CLI

## Check Version

Verify the installed CLI version:

```bash
cargo r -p jito-bam-boost-cli -- --version
```

## How to Claim Subsidy

Validators can claim their allocated JitoSOL rewards by providing a merkle proof. Use the CLI to claim rewards for a specific epoch:

**NOTE: Signer is the validator's identity keypair**

```bash
cargo r -p jito-bam-boost-cli -- \
    bam-boost \
    merkle-distributor \
    claim \
    --network mainnet \
    --epoch <EPOCH> \
    --rpc-url <RPC_URL> \
    --signer <PATH_TO_IDENTITY_KEYPAIR> \
    --commitment confirmed \
    --jito-bam-boost-program-id BoostxbPp2ENYHGcTLYt1obpcY13HE4NojdqNWdzqSSb
```

## Status Dashboard, Batch Claim & TUI

Check all epochs at once, claim everything unclaimed, or use the interactive
full-screen interface. See [docs/claiming.md](./docs/claiming.md).

```bash
# Read-only dashboard (no keypair needed)
cargo r -p jito-bam-boost-cli -- --rpc-url <RPC_URL> --commitment confirmed \
    bam-boost merkle-distributor status --network mainnet --claimant <IDENTITY_PUBKEY>

# Claim every unclaimed epoch
cargo r -p jito-bam-boost-cli -- --rpc-url <RPC_URL> --commitment confirmed \
    --signer <PATH_TO_IDENTITY_KEYPAIR> \
    bam-boost merkle-distributor claim-all --network mainnet

# Interactive TUI
cargo r -p jito-bam-boost-cli -- tui
```
