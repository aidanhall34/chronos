#!make
SHELL:=/bin/bash

RUST_VERSION := $(shell grep 'channel' rust-toolchain.toml | sed 's/.*"\(.*\)"/\1/')
EXPORTER ?= prom
LGTM_IMAGE ?= grafana/otel-lgtm:0.24.1

# pp - pretty print function
yellow := $(shell tput setaf 3)
normal := $(shell tput sgr0)
define pp
	@printf '$(yellow)$(1)$(normal)\n'
endef


help: Makefile
	@echo " Choose a command to run:"
	@sed -n 's/^##//p' $< | column -t -s ':' | sed -e 's/^/ /'


# DEV #############################################################################################

## withenv: 😭 CALL TARGETS LIKE THIS `make withenv RECIPE=dev.init`
withenv:
# NB: IT APPEARS THAT LOADING ENVIRONMENT VARIABLES INTO make SUUUUCKS.
# NB: THIS RECIPE IS A HACK TO MAKE IT WORK.
# NB: THAT'S WHY THIS MAKEFILE NEEDS TO BE CALLED LIKE `make withenv RECIPE=dev.init`
	test -e .env || cp .env.example .env
	bash -c 'set -o allexport; source .env; set +o allexport; make "$$RECIPE"'

## dev.init: 🌏 Initialize local dev environment
# If rdkafka compilation fails with SSL error then install openssl@1.1 or later and export:
# export LDFLAGS=-L/opt/homebrew/opt/openssl@1.1/lib
# export CPPFLAGS=-I/opt/homebrew/opt/openssl@1.1/include
dev.init: install
	$(call pp,install git hooks...)
	cargo test

## dev.kafka_init: 🥁 Init kafka topic
# dev.kafka_init:
# 	$(call pp,creating kafka topic...)
# 	cargo run --example kafka_create_topic

dev.chronos_ex:
	$(call pp,creating kafka topic...)
	cargo run --example chronos_ex

## pg.create: 🥁 Create database
pg.create:
	$(call pp,creating database...)
	cargo run --example pg_create_database

## pg.migrate: 🥁 Run migrations on database
pg.migrate:
	$(call pp,running migrations on database...)
	cargo run --package pg_mig --bin chronos-pg-migrations

# TEST / DEPLOY ###################################################################################

## install: 🧹 Installs dependencies
install:
	$(call pp,pull rust dependencies...)
	rustup install "${RUST_VERSION}"
	rustup component add rust-src clippy llvm-tools-preview
	rustup toolchain install nightly
	rustup override set "${RUST_VERSION}"
	cargo install cargo2junit grcov
	cargo fetch

## build: 🧪 Compiles rust
build:
	$(call pp,build rust...)
	cargo build


## dev.run: 🧪 Runs rust app in watch mode
dev.run:
	$(call pp,run app...)
	cargo  watch -q -c -x 'run --package chronos_bin --bin chronos'

## run: 🧪 Runs rust app
run:
	$(call pp,run app...)
	cargo run --package chronos_bin --bin chronos

## run: 🧪 Runs rust app in release mode
run.release:
	$(call pp,run app...)
	cargo run --package chronos_bin -r --bin chronos


## lint: 🧹 Checks for lint failures on rust
lint:
	$(call pp,lint rust...)
	cargo check
	cargo fmt -- --check
	cargo clippy --all-targets

## test.unit: 🧪 Runs unit tests
test.unit:
	$(call pp,rust unit tests...)
	cargo test

## integration: 🧪 Start deps, migrate, run Chronos, publish test message, verify metrics
integration: build
	$(call pp,running integration test...)
	@bash scripts/integration.sh

## integration.down: 🛑 Stop docker services started by make integration
integration.down:
	$(call pp,stopping integration services...)
	docker compose stop postgres kafka jaeger-all-in-one otel-collector 2>/dev/null || true
	docker compose rm -f postgres kafka jaeger-all-in-one otel-collector 2>/dev/null || true

## metrics.check: 🔍 Verify /metrics endpoint responds (requires running app)
metrics.check:
	$(call pp,check metrics endpoint...)
	curl -sf "http://localhost:$${OTEL_EXPORTER_PROMETHEUS_PORT:-$${METRICS_PORT:-9090}}/metrics" | head -20

## metrics.mock: 🔍 Run Prometheus/OTLP metrics mock example with EXPORTER=prom|otlp
metrics.mock:
	$(call pp,run metrics mock example with exporter $(EXPORTER)...)
	@case "$(EXPORTER)" in \
		prom|prometheus) OTEL_METRICS_EXPORTER=prometheus cargo run --package prom_otlp_mock_runner --bin prom_otlp_mock ;; \
		otlp) OTEL_METRICS_EXPORTER=otlp OTEL_EXPORTER_OTLP_PROTOCOL=grpc cargo run --package prom_otlp_mock_runner --bin prom_otlp_mock ;; \
		*) echo "unsupported EXPORTER=$(EXPORTER); use EXPORTER=prom or EXPORTER=otlp" >&2; exit 2 ;; \
	esac

## lgtm.validate: 🔍 Validate LGTM Prometheus and OpenTelemetry Collector configs
lgtm.validate:
	$(call pp,validate LGTM Prometheus config with $(LGTM_IMAGE)...)
	docker run --rm \
		-v "$(PWD)/dev/prometheus.yaml:/otel-lgtm/prometheus.yaml:ro" \
		--entrypoint /otel-lgtm/prometheus/promtool \
		$(LGTM_IMAGE) check config /otel-lgtm/prometheus.yaml
	$(call pp,validate LGTM OpenTelemetry Collector config with $(LGTM_IMAGE)...)
	docker run --rm \
		-v "$(PWD)/dev/otelcol-contrib.yaml:/otel-lgtm/otelcol-config.yaml:ro" \
		--entrypoint /otel-lgtm/otelcol-contrib/otelcol-contrib \
		$(LGTM_IMAGE) validate --config=file:/otel-lgtm/otelcol-config.yaml --feature-gates=service.profilesSupport

## test.unit.coverage: 🧪 Runs rust unit tests with coverage 'cobertura' and 'junit' reports
test.unit.coverage:
	$(call pp,rust unit tests...)
	sh scripts/coverage-report.sh

## docker.up: 🧪 Runs rust app in docker container along with kafka and postgres
docker.up:
	$(call pp,run app...)
	docker-compose --env-file /dev/null up -d

## docker.down: bring down the docker containers
docker.down:
	$(call pp,run app...)
	docker-compose down
# PHONY ###########################################################################################

# To force rebuild of not-file-related targets, make the targets "phony".
# A phony target is one that is not really the name of a file;
# Rather it is just a name for a recipe to be executed when you make an explicit request.
.PHONY: build
