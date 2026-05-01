## setup: Check local development dependencies and prepare .env
setup:
	$(call pp,checking development dependencies...)
	$(call require_cmd,cargo)
	$(call require_cmd,rustup)
	$(call require_cmd,docker)
	$(call require_cmd,curl)
	$(call require_cmd,awk)
	@test -e .env || cp .env.example .env
	@rustup component list --installed | grep -q '^rustfmt' || { echo 'Missing Rust component: rustfmt. Install with: rustup component add rustfmt' >&2; exit 1; }
	@rustup component list --installed | grep -q '^clippy' || { echo 'Missing Rust component: clippy. Install with: rustup component add clippy' >&2; exit 1; }
	@printf 'Development dependencies look ready.\n'

## withenv: Run a make recipe with variables loaded from .env, for example make withenv RECIPE=run
withenv:
	test -e .env || cp .env.example .env
	bash -c 'set -o allexport; source .env; set +o allexport; make "$$RECIPE"'

## dev.init: Initialize local dev environment
dev.init: setup
	$(call pp,checking rust tests...)
	cargo test

dev.chronos_ex:
	$(call pp,creating kafka topic...)
	cargo run --example chronos_ex

## pg.create: Create database
pg.create:
	$(call pp,creating database...)
	cargo run --example pg_create_database

## pg.migrate: Run migrations on database
pg.migrate:
	$(call pp,running migrations on database...)
	cargo run --package pg_mig --bin chronos-pg-migrations

## run: Run Chronos locally
run:
	$(call pp,run app...)
	cargo run --package chronos_bin --bin chronos

## run.release: Run Chronos locally in release mode
run.release:
	$(call pp,run app...)
	cargo run --package chronos_bin -r --bin chronos

## dev.run: Run Chronos in cargo-watch mode
dev.run:
	$(call pp,run app...)
	cargo watch -q -c -x 'run --package chronos_bin --bin chronos'

.PHONY: setup withenv dev.init dev.chronos_ex pg.create pg.migrate run run.release dev.run
