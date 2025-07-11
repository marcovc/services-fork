![pull request](https://github.com/cowprotocol/services/workflows/pull%20request/badge.svg) ![deploy](https://github.com/cowprotocol/services/workflows/deploy/badge.svg)

# Cow Protocol Services

This repository contains backend code for [Cow Protocol Services](https://docs.cow.fi/) written in Rust.

## Order Book

The `orderbook` crate provides the http api through which users (usually through a frontend web application) interact with the order book.
Users can add signed orders to the order book and query the state of their orders.
They can also use the API to estimate fee amounts and limit prices before placing their order.

Solvers also interact with the order book by querying a list of open orders that they can attempt to settle.

The api is documented with [openapi](https://api.cow.fi/docs/).
A simple example script that uses the API to place random orders can be found in [this repo](https://github.com/cowprotocol/trading-bot)

The order book service itself uses PostgreSQL as a backend to persist orders.
In addition to connecting the http api to the database it also checks order validity based on the block time, trade events, erc20 funding and approval so that solvers can query only valid orders.

Multiple concurrent `orderbook`s can run at the same time, allowing the user-facing API to scale horizontally with increased traffic.

## Autopilot

The `autopilot` crate is responsible for driving the protocol forward.
Concretely, it is responsible for "cutting" new auctions (i.e. determining auction boundaries and which orders are to be included, as well as various parameters important for settlement objective value computation).

The `autopilot` connects to the same PostgreSQL database as the `orderbook` and uses it to query orders as well as storing the most recent auction and settlement competition.

## Other Crates

There are additional crates that live in the cargo workspace.

- `alerter` provides a custom alerter binary that looks at the current orderbook and counts metrics for orders that should be solved but aren't
- `contracts` provides _[ethcontract-rs](https://github.com/gnosis/ethcontract-rs)_ based smart contract bindings
- `database` provides the shared database and storage layer logic shared between the `autopilot` and `orderbook`
- `driver` an in-development binary that intends to replace the `solver`; it has a slightly different design that allows co-location with external solvers
- `e2e` end-to-end tests
- `ethrpc` ethrpc client with a few extensions
- `model` provides the serialization model for orders in the order book api
- `number` extensions to number types, such as numerical conversions between 256-bit integers, nonzero types and de/serialization implementations
- `observe` initialization and helper functions for logging and metrics
- `shared` provides other shared functionality between the solver and order book
- `testlib` shared helpers for writing unit and end-to-end tests

## Testing

The CI (check .github/workflows/pull-request.yaml) runs unit tests, e2e tests, `clippy` and `cargo fmt`. The CI system uses [cargo-nextest](https://nexte.st/) and therefor all tests are getting verified by it. `cargo-nextest` and `cargo test` handle global state slightly differently which can cause some tests to fail with `cargo test`. That's why it's recommended to run tests with `cargo nextest run`.

### Unit Tests:

`cargo nextest run`

### DB Tests:

`cargo nextest run postgres --test-threads 1 --run-ignored ignored-only`

**Note:** Requires postgres database running (see below).

### E2E Tests - Local Node:

`cargo nextest run -p e2e local_node --test-threads 1 --failure-output final --run-ignored ignored-only`.

**Note:** Requires postgres database and local test network with smart contracts deployed (see below).

### E2E Tests - Forked Node:

`FORK_URL=<mainnet archive node RPC URL> cargo nextest run -p e2e forked_node --test-threads 1 --run-ignored ignored-only --failure-output final`.

**Note:** Requires postgres database (see below).

### Clippy

`cargo clippy --all-features --all-targets -- -D warnings`

### Formatting

We rely on custom formatting rules which currently can only be used with a nightly toolchain which is why we need to override the default toolchain with `+nightly`.

`cargo +nightly fmt --all`

### Flaky Tests
In case a test is flaky and only fails **sometimes** in CI you can use the [`run-flaky-test`](.github/workflows/pull-request.yaml) github action to test your fix with the CI to get confidence that the fix that works locally also works in CI.

## Development Setup

### Postgres

The tests that require postgres connect to the default database of a locally running postgres instance on the default port.
To achieve this, open a new shell and run the command below:
Note: The migrations will be applied as well.

```sh
docker-compose up
```

### Local Test Network

In order to run the `e2e` tests you have to have an EVM compatible testnet running locally.
We make use of [anvil](https://github.com/foundry-rs/foundry) from the Foundry project to spin up a local testnet.

`anvil` supports all the RPC methods we need to run the services and tests.

1. Install [foundryup](https://book.getfoundry.sh/getting-started/installation).
2. Install foundry with `foundryup`.
3. Run `anvil` with the following configuration:

```bash
ANVIL_IP_ADDR=0.0.0.0 anvil \
  --gas-price 1 \
  --gas-limit 10000000 \
  --base-fee 0 \
  --balance 1000000 \
  --chain-id 1 \
  --timestamp 1577836800
```

### Profiling

All binaries are compiled with support for [tokio-console](https://github.com/tokio-rs/console) by default to allow you to look inside the tokio runtime.
However, this feature is not enabled at runtime by default because it comes with a pretty significant memory overhead. To enable it you just have to set the environment variable `TOKIO_CONSOLE=true` and run the binary you want to instrument.

You can install and run `tokio-console` with:
```bash
cargo install --locked tokio-console
tokio-console
```


### Changing Log Filters

It's possible to change the tracing log filter while the process is running. This can be useful to debug an error that requires more verbose logs but which might no longer appear after restarting the system.

Each process opens a UNIX socket at `/tmp/log_filter_override_<program_name>_<pid>.sock`. To change the log filter connect to it with `nc -U <path>` and enter a new log filter.
You can also reset the log filter to the filter the program was initially started with by entering `reset`.

See [here](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives) for documentation on the supported log filter format.

## Running the Services Locally

### Prerequisites

Reading the state of the blockchain requires issuing RPC calls to an ethereum node. This can be a testnet you are running locally, some "real" node you have access to or the most convenient thing is to use a third-party service like [infura](https://infura.io/) to get access to an ethereum node which we recommend.
After you made a free infura account they offer you "endpoints" for the mainnet and different testnets. We will refer those as `node-urls`.
Because services are only run on Mainnet, Gnosis Chain, Arbitrum One and Sepolia you need to select one of those.

Note that the `node-url` is sensitive data. The `orderbook` and `solver` executables allow you to pass it with the `--node-url` parameter. This is very convenient for our examples but to minimize the possibility of sharing this information by accident you should consider setting the `NODE_URL` environment variable so you don't have to pass the `--node-url` argument to the executables.

To avoid confusion during your tests, always double-check that the token and account addresses you use actually correspond to the network of the `node-url` you are running the executables with.

### Autopilot

To see all supported command line arguments run `cargo run --bin autopilot -- --help`.

Run an `autopilot` with:

```sh
cargo run --bin autopilot -- \
  --skip-event-sync true \
  --node-url <YOUR_NODE_URL>
```

`--skip-event-sync` will skip some work to speed up the initialization process.

### Orderbook

To see all supported command line arguments run `cargo run --bin orderbook -- --help`.

Run an `orderbook` on `localhost:8080` with:

```sh
cargo run --bin orderbook -- \
  --node-url <YOUR_NODE_URL>
```

If your node supports `trace_callMany`, or you have an additional node with tracing support, consider also specifying `--tracing-node-url <YOUR_NODE_URL>`.
This will enable the tracing-based bad token detection.

Note: Current version of the code does not compile under Windows OS. Context and workaround are [here](https://github.com/cowprotocol/services/issues/226).

### Solvers

To see all supported command line arguments run `cargo run --bin solver -- --help`.

Run a solver which is connected to an `orderbook` at `localhost:8080` with:

```sh
cargo run -p solver -- \
  --solver-account 0xa6DDBD0dE6B310819b49f680F65871beE85f517e \
  --transaction-strategy DryRun \
  --node-url <YOUR_NODE_URL>
```

`--transaction-strategy DryRun` will make the solver only print the solution but not submit it on-chain. This command is absolutely safe and will not use any funds.

The `solver-account` is responsible for signing transactions. Solutions for settlements need to come from an address the settlement contract trusts in order to make the contract actually consider the solution. If we pass a public address, like we do here, the solver only pretends to be used for testing purposes. To actually submit transactions on behalf of a solver account you would have to pass a private key of an account the settlement contract trusts instead. Adding your personal solver account is quite involved and requires you to get in touch with the team, so we are using this public solver address for now.

To make things more interesting and see some real orders you can connect the `solver` to our real `orderbook` service. There are several orderbooks for production and staging environments on different networks. Find the `orderbook-url` corresponding to your `node-url` which suits your purposes and connect your solver to it with `--orderbook-url <URL>`.

The publicly available orderbook URLs can be found [here](https://api.cow.fi/docs/).

Always make sure that the `solver` and the `orderbook` it connects to are configured to use the same network.

### Frontend

To conveniently submit orders checkout the [CowSwap](https://github.com/cowprotocol/cowswap) frontend and point it to your local instance.
