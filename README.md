# tbtc4u

Pro-bono prover for tBTC Deposits. Run this software to be a kind person.

This is HACKATHON SOFTWARE. Don't trust it with mainnet funds. Ropsten only!

tbtc4u watches for Deposits that aren't being set up properly, and helps them
along. It helps people avoid losing BTC due to operator error, by proving BTC
funding to tBTC deposits if the owner doesn't.

tbtc4u uses `ethers-rs` and `riemann-rs` providers to run chain eth and bitcoin
chain polling, and tracks deposits through several states. It watches for a
specific critical failure, and tries to help fix it

# Requirements:

Clone `riemann-rs` on the `provider` branch alongside this repo

# Goals:

- [x] Listen to TBTC on ropsten
- [x] Index deposits being set up
- [x] Watch BTC for funding txns
- [x] Send funding proofs to ropsten
- [x] Run as CLI/daemon
- [x] User-facing data output

## Less-important Goals

- [ ] Config options (e.g. infura key)
- [ ] Sweep funds held by hot key
