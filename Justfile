default:
    just --choose

# lint the code
@lint:
    cargo fmt --all
    cargo clippy --all-targets --all-features

@test:
    cargo test --workspace -- --quiet

# update dependency versions, and checks for outdated ones
@update-deps:
   cargo update
   command -v cargo-outdated > /dev/null || (echo "cargo-outdated is not installed" && exit 1)
   cargo outdated


# list unused dependencies
@unused-deps:
    command -v cargo-udeps >/dev/null || (echo "cargo-udeps not installed" && exit 1)
    cargo +nightly udeps

@versions:
    rustc --version
    cargo --version

@check:
    cargo check


@release:
    cargo build --release --verbose


# run unit tests (in release mode)
@test-release:
    cargo test --workspace --release --verbose


@run:
    cargo run --package=kurtex_cli


@dev:
    command -v cargo-watch > /dev/null || (echo "cargo-watch is not installed" && exit 1)
    cargo watch -x "run --package=kurtex_cli"

# run github actions ci locally
@ci:
    command -v act > /dev/null || (echo "act is not installed. see https://nektosact.com" && exit 1)
    act
