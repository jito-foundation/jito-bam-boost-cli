# Jito BAM Boost CLI

## Dashboard

View the BAM Boost program statistics and validator participation on the Jito BAM Boost Dashboard:

https://jito.retool.com/apps/4f86ad18-f57c-11f0-b25f-8bc3bda172db/Jito%20BAM%20Boost%20Mainnet/page1

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
    --signer <PATH_TO_KEYPAIR> \
    --commitment confirmed \
    --jito-bam-boost-program-id BoostxbPp2ENYHGcTLYt1obpcY13HE4NojdqNWdzqSSb
```
