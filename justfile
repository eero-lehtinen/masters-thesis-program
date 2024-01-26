set shell := ["sh", "-c"]
set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

vis *ARGS:
	cargo run --release -- {{ARGS}}

bench *ARGS:
	cargo run --release --features bench -- {{ARGS}}

run-trace *ARGS:
	cargo run --release --features bevy/trace_chrome -- {{ARGS}}

fmt:
	cargo fmt

lint:
	cargo clippy --target-dir target/rust-analyzer

lint-fix:
	cargo clippy --target-dir target/rust-analyzer --fix --allow-dirty --allow-staged

