set shell := ["sh", "-c"]
set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

# E.g. `just viewer -l levels/a.level`
viewer *ARGS:
	cargo run --release -- viewer {{ARGS}}

bench *ARGS:
	cargo run --release -- bench {{ARGS}}

editor *ARGS:
	cargo run --release --  editor {{ARGS}}

run-trace *ARGS:
	cargo run --release --features bevy/trace_chrome -- {{ARGS}}

fmt:
	cargo fmt

lint:
	cargo clippy --target-dir target/rust-analyzer

lint-fix:
	cargo clippy --target-dir target/rust-analyzer --fix --allow-dirty --allow-staged

