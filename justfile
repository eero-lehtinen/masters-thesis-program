set shell := ["sh", "-c"]
set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

run *ARGS:
	cargo run --release -- {{ARGS}}

run-trace *ARGS:
	cargo run --release --features bevy/trace_chrome -- {{ARGS}}

fmt:
	cargo fmt

lint:
	cargo clippy

lint-fix:
	cargo clippy  --fix --allow-dirty --allow-staged

