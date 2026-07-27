# Claiming Your BAM Boost Subsidy

This guide walks Solana validator operators through checking, claiming, and
verifying Jito BAM Boost subsidy payouts using `jito-bam-boost-cli` — either
via individual CLI commands or the interactive full-screen TUI.

## 1. Summary

1. **Install / build** the CLI from source (Rust toolchain required).
2. **Check status** — run `status` to see every epoch's eligibility, amount,
   and claimed/unclaimed state for your validator identity. This is
   read-only and needs only your **pubkey**, not your keypair.
3. **Claim** — either:
   - claim one epoch at a time with `claim`, or
   - claim every unclaimed epoch in one shot with `claim-all`, or
   - use the interactive `tui` for a guided scan → select → claim flow.
4. **Verify** — confirm a claim landed with `claim-status get`, which reads
   the on-chain `ClaimStatus` PDA for that epoch/claimant pair.

## 2. Prerequisites

- **Rust toolchain** matching [`rust-toolchain.toml`](../rust-toolchain.toml)
  (currently `1.89.0`, installed automatically by `cargo`/`rustup` when you
  build in this repo).
- **Your validator identity keypair** (the same key that signs votes/blocks)
  is the CLI's `--signer` for any command that submits a transaction
  (`claim`, `claim-all`, and the TUI's claim step).

  > **⚠️ SECURITY WARNING:** The signer is your **validator identity
  > keypair** — the same key that controls your stake and voting.
  > - Keep it on a secure host; do not copy it to a workstation or laptop
  >   just to run `claim-all` or the TUI.
  > - Prefer running claim commands directly on (or via a hardened jump
  >   host into) the validator machine that already holds the key.
  > - **`status` checks require only your public key** (`--claimant
  >   <PUBKEY>`) — never your keypair. Use `status` freely, including from
  >   a laptop or CI, to monitor subsidies without ever touching the
  >   signer.
- A Solana **RPC endpoint** (`--rpc-url`). The default is
  `https://api.mainnet-beta.solana.com`, but see
  [Troubleshooting](#8-troubleshooting) for rate-limit guidance — a private
  or dedicated RPC endpoint is strongly recommended for anything beyond
  occasional manual checks.

## 3. Check your subsidies

The `status` command scans every published epoch and reports whether your
validator identity was eligible, and if so, whether the subsidy has already
been claimed. It only needs your **claimant pubkey** — no keypair required.

```bash
cargo r -p jito-bam-boost-cli -- \
    --rpc-url <RPC_URL> \
    --commitment confirmed \
    bam-boost merkle-distributor status \
    --network mainnet \
    --claimant <IDENTITY_PUBKEY>
```

Sample output:

```
   Epoch    Amount (JitoSOL)  Status
     993         0.750000000  expired
     994                   -  not eligible
     995         1.250000000  claimed
     996         0.980000000  unclaimed
     997                   -  not eligible
     998         2.010000000  unclaimed

Claimed: 1 epochs, 1.250000000 JitoSOL | Unclaimed: 2 epochs, 2.990000000 JitoSOL | Expired: 1 epochs, 0.750000000 JitoSOL
```

The stats line at the bottom is a running total across every scanned epoch —
not just the ones printed above it — broken down by claimed, unclaimed
(still claimable), and expired amounts. The same line appears as the first
line of the TUI dashboard's footer (Section 6) and is printed by `claim-all`
right after its scan (Section 5).

### Expired epochs

Each BAM Boost distribution's claim window is limited: on mainnet the
window has been observed to last roughly 9 epochs after a distribution is
published, after which the on-chain distributor's token vault is swept and
any unclaimed allocation for that epoch is no longer claimable. `status`
reports this as `expired` (never as `unclaimed`), and `claim-all` reports
it as expired lamports in the stats line and skips it — expired epochs are
never included in the list of epochs `claim-all` attempts to claim.

**Detection is on-chain, not a hardcoded epoch count.** The CLI does not
assume any fixed window length. An epoch is marked `expired` only when it
has an allocation, that allocation has not been claimed, *and* the
distributor's associated token vault account is empty (missing entirely, or
present with a zero SPL token balance) — i.e. the sweep has observably
already happened for that specific epoch. The "~9 epochs" figure above is
an observed operational characteristic, not a value the CLI checks against;
if the sweep timing ever changes, `status`/`claim-all` will still classify
epochs correctly because they check the vault directly.

If you see `expired` for an epoch you believe you were owed, there is
nothing to claim — the subsidy for that epoch can no longer be paid out
through this CLI. This is expected behavior for allocations left unclaimed
past the claim window, not a bug.

Add `--print-json` for machine-readable output (handy for monitoring or
scripting):

```bash
cargo r -p jito-bam-boost-cli -- \
    --rpc-url <RPC_URL> \
    --commitment confirmed \
    bam-boost merkle-distributor status \
    --network mainnet \
    --claimant <IDENTITY_PUBKEY> \
    --print-json
```

## 4. Claim: one epoch

To claim a single epoch's subsidy (existing `claim` command):

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

`--jito-bam-boost-program-id` is optional — it defaults to the deployed BAM
Boost program ID and only needs to be overridden for a non-standard
deployment (e.g. local validator testing).

## 5. Claim: everything at once

`claim-all` scans for every unclaimed epoch for the signer's pubkey, shows a
summary table, asks for confirmation, then claims each epoch in turn:

```bash
cargo r -p jito-bam-boost-cli -- \
    --rpc-url <RPC_URL> \
    --commitment confirmed \
    --signer <PATH_TO_IDENTITY_KEYPAIR> \
    bam-boost merkle-distributor claim-all \
    --network mainnet
```

Sample interaction:

```
Claimed: 1 epochs, 1.250000000 JitoSOL | Unclaimed: 2 epochs, 2.990000000 JitoSOL | Expired: 1 epochs, 0.750000000 JitoSOL
Skipping 1 expired epoch(s) (0.750000000 JitoSOL no longer claimable)
Unclaimed epochs for <IDENTITY_PUBKEY>:
  epoch    996: 0.980000000 JitoSOL
  epoch    998: 2.010000000 JitoSOL
Total: 2.990000000 JitoSOL across 2 epoch(s)
Proceed with claiming? [y/N] y
epoch 996: claiming...
epoch 996: OK  3sTz9k8...signature...
epoch 998: claiming...
epoch 998: OK  5Qp1m2...signature...

Done: 2 claimed, 0 failed, 0 other
```

The stats line is printed immediately after the scan completes, before the
list of epochs claim-all is about to claim. The "Skipping N expired
epoch(s)" line only appears when at least one epoch is expired — expired
epochs are never included in the unclaimed list below it, so `claim-all`
never attempts to claim them (they are not resubmittable — see
[Expired epochs](#expired-epochs) below).

Pass `--yes` to skip the interactive confirmation prompt (useful for cron
jobs or automation once you're comfortable with the flow):

```bash
cargo r -p jito-bam-boost-cli -- \
    --rpc-url <RPC_URL> --commitment confirmed \
    --signer <PATH_TO_IDENTITY_KEYPAIR> \
    bam-boost merkle-distributor claim-all --network mainnet --yes
```

> **⚠️ `--print-tx` is not a dry run.** The global `--print-tx` flag is
> documented as "print out the raw TX instead of running it," but in the
> current build `process_transaction` does not check this flag — it always
> signs and **broadcasts** the transaction regardless of `--print-tx`. This
> is a known upstream bug (fix pending). Do **not** rely on `--print-tx` to
> preview a claim without sending it; if you want to see amounts before
> committing, use `status` or `claim-all` without `--yes` (its own
> confirmation prompt), not `--print-tx`.

## 6. TUI workflow

For an interactive, full-screen experience that combines scanning, batch
selection, and claiming in one session, launch the TUI:

```bash
cargo r -p jito-bam-boost-cli -- tui
```

You can pre-fill the network, RPC URL, and (via `--signer`) both the
claimant pubkey and keypair path used for confirmation:

```bash
cargo r -p jito-bam-boost-cli -- \
    --rpc-url <RPC_URL> \
    --signer <PATH_TO_IDENTITY_KEYPAIR> \
    tui --network mainnet
```

### Screens

1. **Setup** — enter/confirm the network and claimant pubkey, then start a
   scan.
2. **Dashboard** — a bordered **Unclaimed** panel at the top lists only the
   epochs you can actually claim right now (unclaimed, not expired), newest
   first; move the cursor and toggle selection there. Below it, a bordered,
   read-only **All epochs** panel shows the full scan history (every status,
   including `claimed` and `expired`), also newest first, paginated
   `PAGE_SIZE` (20) rows at a time. A footer shows the running
   claimed/unclaimed/expired stats line plus the key hints.
3. **Confirm** — a popup asking you to type/confirm the signer keypair path
   and confirm before anything is sent.
4. **Progress** — live per-epoch claim results as the batch runs.

### Keyboard reference

| Screen | Key | Action |
|---|---|---|
| Setup | `Tab` / `↑` `↓` | Move focus between fields |
| Setup | `←` / `→` / `Space` | Toggle network (only while the Network field is focused) |
| Setup | `Enter` (on `[Start scan]`) | Start scanning epochs |
| Setup | `Esc` | Quit |
| Dashboard | `↑` `↓` | Move the cursor within the **Unclaimed** panel |
| Dashboard | `Space` | Toggle selection on the epoch under the cursor |
| Dashboard | `a` | Select all unclaimed (claimable) epochs — expired epochs are never selected |
| Dashboard | `c` | Open the Confirm popup for the current selection |
| Dashboard | `←` `→` (or `PgUp`/`PgDn`) | Page the **All epochs** history panel; selection is unaffected — paging only changes what the read-only history panel displays |
| Dashboard | `r` | Rescan |
| Dashboard | `q` | Quit |
| Confirm | `y` | Confirm and start claiming (requires a keypair path already in the field) |
| Confirm | `n` / `Esc` | Cancel, return to Dashboard |
| Progress | `b` (once claims finish) | Go back to Dashboard and rescan |
| Progress | `q` (once claims finish) | Quit |

> **Confirm popup note:** the keypair-path field on the Confirm screen
> accepts free text, and `y`/`n` are reserved as the confirm/cancel keys —
> so you **cannot type a literal `y` or `n` character into the path field**.
> Pre-fill the keypair path by passing `--signer <PATH_TO_IDENTITY_KEYPAIR>`
> on the command line before launching the TUI; the field will already be
> populated when you reach the Confirm screen, so pressing `y` immediately
> confirms without needing to type anything.

## 7. Verification

After claiming (via `claim`, `claim-all`, or the TUI), verify a specific
epoch landed by reading its `ClaimStatus` account:

```bash
cargo r -p jito-bam-boost-cli -- \
    --rpc-url <RPC_URL> --commitment confirmed \
    bam-boost claim-status get \
    --epoch <EPOCH> \
    --claimant <IDENTITY_PUBKEY>
```

Sample output:

```
ClaimStatus PDA: 7fH2k9...address...
{
  "discriminator": [22, 183, 249, 157, 247, 95, 150, 96],
  "claimant": "<IDENTITY_PUBKEY>",
  "amount": 980000000
}
```

**PDA semantics: the account's existence *is* the claim receipt.** There is
no "unclaimed" state for this account — either the `ClaimStatus` PDA has
been created (meaning the claim succeeded and `amount` shows what was paid,
in lamports of JitoSOL), or the RPC call fails because the account does not
exist (meaning that epoch has not been claimed by that claimant yet). Use
`status` (Section 3) to see unclaimed epochs across the board; use
`claim-status get` to confirm one specific epoch after claiming.

## 8. Troubleshooting

- **"Claim status account already exists — subsidy for this epoch has
  already been claimed."** — You (or someone with this signer) already
  claimed this epoch. This is not an error to act on; run `status` to
  confirm, or `claim-status get` for that epoch to see the recorded amount.
- **Claimant not in tree ("claimant not in tree" during `claim-all`, or
  `-` / "not eligible" in `status`)** — Your validator identity was not
  included in that epoch's merkle distribution. This usually means you
  were not eligible for a subsidy that epoch (e.g. did not meet the BAM
  criteria for that period). This is expected behavior, not a bug.
- **RPC errors / HTTP 429 (rate limited)** — The public
  `https://api.mainnet-beta.solana.com` endpoint enforces aggressive rate
  limits, especially when `status` scans many epochs in quick succession.
  Use a private or dedicated RPC endpoint via `--rpc-url` (or
  `--config-file`) for anything beyond occasional manual checks.
- **GCS 404 for an epoch** — The scanner treats an HTTP 404 from the
  merkle-tree distribution bucket as "no distribution published for that
  epoch" — it is not an error. It surfaces the same as "not eligible" in
  `status` output. This is normal for epochs before BAM Boost started, or
  epochs where no distribution round has been published yet.
- **Corrupted local cache** — The scanner caches downloaded merkle-tree
  data locally (by default under your platform's cache directory, e.g.
  `~/.cache/jito-bam-boost/` on Linux) to avoid re-downloading on every
  `status`/`claim-all` run. If a cache file is corrupted, the scanner
  automatically detects the parse failure, deletes the bad file, and
  re-fetches from the network — no action needed. If you ever want to
  force a full refresh, the cache directory is safe to delete entirely;
  it will be rebuilt on the next scan.
