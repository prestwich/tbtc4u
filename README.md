# Goals:

1. Listen to TBTC on ropsten
2. Index deposits being set up
3. Watch BTC for funding txns
4. Send funding proofs to ropsten
5. Run as CLI/daemon
6. User-facing data output

## Less-important Goals

1. Config options (e.g. infura key)
2. Sweep funds held by hot key

## Tasks

- [x] 1. Check if ethers supports websockets
- [x] 2. Provider for node RPC
- [x] 3. Figure out deposit logging contract address on ropsten
    1. 0x14dC06F762E7f4a756825c1A1dA569b3180153cB -- TBTC system
- [ ] 4. Figure out how to trigger deposits on ropsten
- [ ] 5. Write something to get SPV proofs from a provider
