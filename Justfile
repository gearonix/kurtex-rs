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

@watch-check:
    command -v cargo-watch > /dev/null || (echo "cargo-watch is not installed" && exit 1)
    cargo watch -x "check"


@release:
    cargo build --release --verbose


# run unit tests (in release mode)
@test-release:
    cargo test --workspace --release --verbose


@run:
    cargo run --bin=ktx

@debug:
    RUST_BACKTRACE=full cargo run --bin=ktx

@watch:
    command -v cargo-watch > /dev/null || (echo "cargo-watch is not installed" && exit 1)
    cargo watch -x "run --bin=ktx"

# run github actions ci locally
@ci:
    command -v act > /dev/null || (echo "act is not installed. see https://nektosact.com" && exit 1)
    act

@log:
  mkdir -p dev
  cargo run --bin=ktx 2>dev/stdout.log 1> dev/stdout.log


@debug-log:
  mkdir -p dev
  RUST_BACKTRACE=full cargo run --bin=ktx 2>dev/stderr.log 1> dev/stdout.log

alias r:=run
alias d:=debug
alias w:=watch
alias l:=lint
alias c:=check