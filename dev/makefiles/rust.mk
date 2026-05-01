EXPORTER ?= prom
WEAVER_VERSION ?= 0.23.0
WEAVER_IMAGE ?= otel/weaver:v$(WEAVER_VERSION)
WEAVER_REGISTRY ?= examples/weaver/registry
WEAVER_TEMPLATES ?= examples/weaver/templates
WEAVER_GENERATED_DIR ?= chronos_bin/src/metrics/generated
WEAVER_LIVE_CHECK_PORT ?= 4319
WEAVER_LIVE_CHECK_ADMIN_PORT ?= 4320
WEAVER_LIVE_CHECK_OUT ?= /tmp/chronos-weaver-live-check

## build: Build Rust binaries
build: weaver.generate
	$(call pp,build rust...)
	cargo build

## fmt: Format Rust code
fmt:
	$(call pp,format rust...)
	cargo fmt

## lint: Check Rust formatting, clippy, and cargo check
lint:
	$(call pp,lint rust...)
	RUSTFLAGS="-D warnings" cargo check
	cargo fmt -- --check
	RUSTFLAGS="-D warnings" cargo clippy --all-targets -- -D warnings

## test: Run Rust unit tests
test: test.unit

## test.unit: Run Rust unit tests
test.unit:
	$(call pp,rust unit tests...)
	RUSTFLAGS="-D warnings" cargo test

## pre-commit: Run pre-commit checks
pre-commit: lint test.unit

## test.unit.coverage: Run Rust unit tests with coverage reports
test.unit.coverage:
	$(call pp,rust unit tests...)
	sh scripts/coverage-report.sh

## metrics.check: Verify /metrics endpoint responds
metrics.check:
	$(call pp,check metrics endpoint...)
	curl -sf "http://localhost:$${OTEL_EXPORTER_PROMETHEUS_PORT:-$${METRICS_PORT:-9090}}/metrics" | head -20

## metrics.mock: Run Prometheus/OTLP metrics mock example with EXPORTER=prom|otlp
metrics.mock:
	$(call pp,run metrics mock example with exporter $(EXPORTER)...)
	@case "$(EXPORTER)" in \
		prom|prometheus) OTEL_METRICS_EXPORTER=prometheus OTEL_EXPORTER_PROMETHEUS_HOST=$${OTEL_EXPORTER_PROMETHEUS_HOST:-127.0.0.1} OTEL_EXPORTER_PROMETHEUS_PORT=$${OTEL_EXPORTER_PROMETHEUS_PORT:-9092} cargo run --package prom_otlp_mock_runner --bin prom_otlp_mock ;; \
		otlp) OTEL_SERVICE_NAME=chronos-metrics-mock OTEL_RESOURCE_ATTRIBUTES=service.instance.id=chronos-metrics-mock-local OTEL_METRICS_EXPORTER=otlp OTEL_EXPORTER_OTLP_PROTOCOL=grpc OTEL_EXPORTER_OTLP_METRICS_ENDPOINT=$${OTEL_EXPORTER_OTLP_METRICS_ENDPOINT:-http://127.0.0.1:4317} OTEL_METRIC_EXPORT_INTERVAL=$${OTEL_METRIC_EXPORT_INTERVAL:-1000} cargo run --package prom_otlp_mock_runner --bin prom_otlp_mock ;; \
		*) echo "unsupported EXPORTER=$(EXPORTER); use EXPORTER=prom or EXPORTER=otlp" >&2; exit 2 ;; \
	esac

## weaver.check: Validate the Chronos Weaver registry
weaver.check:
	$(call pp,check Weaver registry with $(WEAVER_IMAGE)...)
	docker run --rm -v "$(PWD):/work" -w /work $(WEAVER_IMAGE) registry check -r $(WEAVER_REGISTRY)

## weaver.generate.rust: Generate Rust metric definitions with Weaver
weaver.generate.rust:
	$(call pp,generate Rust metric definitions with $(WEAVER_IMAGE)...)
	docker run --rm -v "$(PWD):/work" -w /work $(WEAVER_IMAGE) registry generate -r $(WEAVER_REGISTRY) --templates $(WEAVER_TEMPLATES) rust $(WEAVER_GENERATED_DIR)
	rustfmt --config-path rustfmt.toml $(WEAVER_GENERATED_DIR)/chronos_metric_definitions.rs

## weaver.generate.markdown: Generate Chronos metrics markdown docs with Weaver
weaver.generate.markdown:
	$(call pp,generate metrics markdown docs with $(WEAVER_IMAGE)...)
	docker run --rm -v "$(PWD):/work" -w /work $(WEAVER_IMAGE) registry generate -r $(WEAVER_REGISTRY) --templates $(WEAVER_TEMPLATES) markdown $(WEAVER_GENERATED_DIR)

## weaver.generate.json-schema: Generate Weaver resolved-registry JSON schema
weaver.generate.json-schema:
	$(call pp,generate Weaver JSON schema with $(WEAVER_IMAGE)...)
	mkdir -p $(WEAVER_GENERATED_DIR)
	docker run --rm -v "$(PWD):/work" -w /work $(WEAVER_IMAGE) registry json-schema -o $(WEAVER_GENERATED_DIR)/resolved-registry.schema.json

## weaver.generate: Generate all Weaver artifacts
weaver.generate: weaver.generate.rust weaver.generate.markdown weaver.generate.json-schema

## weaver.live-check: Run Weaver live-check against the OTLP metrics mock
weaver.live-check:
	$(call pp,run Weaver live-check against metrics mock...)
	@set -euo pipefail; \
	cargo build --package prom_otlp_mock_runner; \
	rm -rf "$(WEAVER_LIVE_CHECK_OUT)"; \
	mkdir -p "$(WEAVER_LIVE_CHECK_OUT)"; \
	docker run --rm --network host \
		-v "$(PWD):/work" \
		-v "$(WEAVER_LIVE_CHECK_OUT):/out" \
		-w /work \
		$(WEAVER_IMAGE) registry live-check \
		-r $(WEAVER_REGISTRY) \
		--input-source otlp \
		--otlp-grpc-address 127.0.0.1 \
		--otlp-grpc-port $(WEAVER_LIVE_CHECK_PORT) \
		--admin-port $(WEAVER_LIVE_CHECK_ADMIN_PORT) \
		--inactivity-timeout 5 \
		--no-stream \
		--format json \
		-o /out & \
	live_check_pid=$$!; \
	trap 'kill "$$live_check_pid" 2>/dev/null || true' EXIT; \
	sleep 2; \
	OTEL_SERVICE_NAME=chronos-metrics-mock \
	OTEL_RESOURCE_ATTRIBUTES=service.instance.id=chronos-metrics-mock-live-check \
	OTEL_METRICS_EXPORTER=otlp \
	OTEL_EXPORTER_OTLP_PROTOCOL=grpc \
	OTEL_EXPORTER_OTLP_METRICS_ENDPOINT=http://127.0.0.1:$(WEAVER_LIVE_CHECK_PORT) \
	OTEL_METRIC_EXPORT_INTERVAL=500 \
	timeout -s INT 10 cargo run --quiet --package prom_otlp_mock_runner --bin prom_otlp_mock || test "$$?" -eq 124; \
	wait "$$live_check_pid"; \
	find "$(WEAVER_LIVE_CHECK_OUT)" -maxdepth 1 -type f -print

.PHONY: build fmt lint test test.unit pre-commit test.unit.coverage metrics.check metrics.mock weaver.check weaver.generate.rust weaver.generate.markdown weaver.generate.json-schema weaver.generate weaver.live-check
