set shell := ["sh", "-c"]
set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

dev *ARGS:
	cargo run --features navigation1

run *ARGS:
	cargo run --release --features navigation1 -- {{ARGS}}

run-trace *ARGS:
	cargo run --release --features --features navigation1 bevy/trace_chrome -- {{ARGS}}

fmt:
	cargo fmt

lint:
	cargo clippy

lint-fix:
	cargo clippy  --fix --allow-dirty --allow-staged

