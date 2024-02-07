set shell := ["sh", "-c"]
set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

# E.g. `just viewer levels/a.level`
viewer *ARGS:
	cargo run --release -- {{ARGS}} viewer

bench *ARGS:
	cargo run --release  -- {{ARGS}} bench

editor *ARGS:
	cargo run --release -- {{ARGS}} editor

run-trace *ARGS:
	cargo run --release --features bevy/trace_chrome -- {{ARGS}}

fmt:
	cargo fmt

lint:
	cargo clippy --target-dir target/rust-analyzer

lint-fix:
	cargo clippy --target-dir target/rust-analyzer --fix --allow-dirty --allow-staged

