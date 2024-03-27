# Boost Guard

Work in progress!

The Boost Guard is in charge of attesting that a user is indeed eligible to claim a boost (whether via vote incentive or vote bribe).

To run your own instance:
1. Make sure you have cargo and Rust installed: https://www.rust-lang.org/tools/install
2. Clone the repository `gcl https://github.com/snapshot-labs/boost-guard.git`
3. Move to the directory `cd boost-guard`
4. Create your [.env](#.env) file
5. Run the client: `cargo run --release`
6. Profit!

## .env

The following variable environment are required for the guard to run:
- `MAINNET_SUBGRAPH_URL`: The url to the mainnet subgraph
- `SEPOLIA_SUBGRAPH_URL`: The url to the sepolia subgraph
- `PRIVATE_KEY`: The guard private key
- `BOOST_NAME`: The boost name used for EIP712 signature (should match the onchain name)
- `BOOST_VERSION`: The boost version used for EIP712 signature (should match the onchain version)
- `VERIFYING_CONTRACT`: The onchain boost address
- `SLOT_URL`: The URL to `/api/v1/slot/` of an eth2 node
- `EPOCH_URL`: The URL to `/api/v1/epoch/` of an eth2 node
- `DATABASE_URL`: A read-only URL acces to the hub's database
- `BEACONCHAIN_API_KEY`: API key to your beaconcha.in account (can be empty if you use your own eth2 node)

Please feel free to read the [docs](https://docs.snapshot.org/user-guides/boost).
