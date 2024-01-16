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
- `HUB_URL`: The url to the snapshot hub
- `SUBGRAPH_URL`: The url of the subgraph
- `PRIVATE_KEY`: The guard private key
- `BOOST_NAME`: The boost name used for EIP712 signature (should match the onchain name)
- `BOOST_VERSION` The boost version used for EIP712 signature (should match the onchain version)
- `VERIFYING_CONTRACT` The onchain boost address

## API

todo

Please feel free to read the [docs](https://docs.boost.limo/).
