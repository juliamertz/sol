lint:
	cargo clippy --fix --all && cargo fmt --all

check:
	cargo check --all

build:
	cargo build --bin solc --release

clean:
	cargo clean
