# Oracle

Based on the Oracle specification [https://spec.interlay.io/spec/oracle.html](https://spec.interlay.io/spec/oracle.html).

## Installation

Run `cargo build` from the root folder of this directory.

## Testing

Run `cargo test` from the root folder of this directory.

## Runtime Integration

### Runtime `Cargo.toml`

To add this pallet to your runtime, simply include the following to your runtime's `Cargo.toml` file:

```TOML
[dependencies.btc-relay]
default_features = false
git = '../creates/oracle'
```

Update your runtime's `std` feature to include this pallet:

```TOML
std = [
    # --snip--
    'oracle/std',
]
```

### Runtime `lib.rs`

You should implement it's trait like so:

```rust
/// Used for test_module
impl oracle::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnExchangeRateChange = ();
    type WeightInfo = ();
    type MaxNameLength = ConstU32<255>;
}
```

and include it in your `construct_runtime!` macro:

```rust
Oracle: oracle::{Module, Call, Config<T>, Storage, Event<T>},
```

## Reference Docs

You can view the reference docs for this pallet by running:

```
cargo doc --open
```
