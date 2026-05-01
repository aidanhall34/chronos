K6_VERSION ?= 1.7.1
XK6_KAFKA_VERSION ?= latest
K6_IMAGE ?= chronos-k6:$(K6_VERSION)
K6_LOG_DIR ?= $(PWD)/dev/lgtm/runtime-logs
K6_RUN_ID ?= chronos-k6-$(shell date +%Y%m%d%H%M%S)
K6_CI_OTLP_ENDPOINT ?= host.docker.internal:4317
K6_DEFAULT_OTEL_ENDPOINT := $(if $(and $(GITHUB_ACTIONS),$(if $(ACT),,1)),$(K6_CI_OTLP_ENDPOINT),lgtm:4317)
K6_OTEL_GRPC_EXPORTER_ENDPOINT ?= $(K6_DEFAULT_OTEL_ENDPOINT)
K6_DOCKER_NETWORK ?= $(if $(and $(GITHUB_ACTIONS),$(if $(ACT),,1)),bridge,chronos)
K6_FULL_LOAD ?= false
K6_LOAD_DEFAULT_RATE := $(if $(filter true 1 yes,$(K6_FULL_LOAD)),1000,100)
K6_LOAD_DEFAULT_DURATION := $(if $(filter true 1 yes,$(K6_FULL_LOAD)),1m,1m)
K6_LOAD_DEFAULT_CONSUME_DURATION := $(if $(filter true 1 yes,$(K6_FULL_LOAD)),2m,90s)
K6_LOAD_PROFILE := $(if $(filter true 1 yes,$(K6_FULL_LOAD)),full load,load)
K6_COMMON_ENV := \
	-e KAFKA_BROKERS=$${KAFKA_BROKERS:-kafka:9092} \
	-e KAFKA_IN_TOPIC=$${KAFKA_IN_TOPIC:-chronos.in} \
	-e KAFKA_OUT_TOPIC=$${KAFKA_OUT_TOPIC:-chronos.out} \
	-e K6_OTEL_SERVICE_NAME=$${K6_OTEL_SERVICE_NAME:-k6-chronos} \
	-e K6_OTEL_METRIC_PREFIX=$${K6_OTEL_METRIC_PREFIX:-k6_} \
	-e K6_OTEL_GRPC_EXPORTER_INSECURE=$${K6_OTEL_GRPC_EXPORTER_INSECURE:-true} \
	-e K6_OTEL_GRPC_EXPORTER_ENDPOINT=$(K6_OTEL_GRPC_EXPORTER_ENDPOINT) \
	-e K6_RUN_ID=$(K6_RUN_ID)
K6_DOCKER_RUN := docker run --rm --network $(K6_DOCKER_NETWORK) --add-host=host.docker.internal:host-gateway -v "$(PWD)/dev/k6:/scripts:ro" -v "$(K6_LOG_DIR):/data/lgtm/logs" $(K6_COMMON_ENV)

## k6.build: Build the custom k6 image with xk6-kafka
k6.build:
	$(call pp,building k6 image $(K6_IMAGE) with k6 $(K6_VERSION) and xk6-kafka $(XK6_KAFKA_VERSION)...)
	docker build -f docker/Dockerfile.k6 --build-arg K6_VERSION=$(K6_VERSION) --build-arg XK6_KAFKA_VERSION=$(XK6_KAFKA_VERSION) -t $(K6_IMAGE) .

## k6.contract: Run the k6 Chronos contract integration test with OTLP output
k6.contract:
	$(call pp,running k6 contract test with OTLP endpoint $(K6_OTEL_GRPC_EXPORTER_ENDPOINT)...)
	mkdir -p "$(K6_LOG_DIR)"
	$(K6_DOCKER_RUN) --entrypoint bash $(K6_IMAGE) -lc 'k6 run --out opentelemetry /scripts/contract.js 2>&1 | tee -a /data/lgtm/logs/k6-contract.jsonl; exit $${PIPESTATUS[0]}'

## k6.load: Run the k6 Chronos load test with OTLP output. Use K6_FULL_LOAD=true for the 1,000 rps full load profile
k6.load:
	$(call pp,running k6 $(K6_LOAD_PROFILE) test with OTLP endpoint $(K6_OTEL_GRPC_EXPORTER_ENDPOINT)...)
	mkdir -p "$(K6_LOG_DIR)"
	$(K6_DOCKER_RUN) \
		-e K6_LOAD_RATE=$${K6_LOAD_RATE:-$(K6_LOAD_DEFAULT_RATE)} \
		-e K6_LOAD_DURATION=$${K6_LOAD_DURATION:-$(K6_LOAD_DEFAULT_DURATION)} \
		-e K6_LOAD_CONSUME_DURATION=$${K6_LOAD_CONSUME_DURATION:-$(K6_LOAD_DEFAULT_CONSUME_DURATION)} \
		-e K6_LOAD_DELAY_MS=$${K6_LOAD_DELAY_MS:-1000} \
		-e K6_LOAD_IMMEDIATE_DELAY_MS=$${K6_LOAD_IMMEDIATE_DELAY_MS:--1000} \
		-e K6_LOAD_IMMEDIATE_RATIO=$${K6_LOAD_IMMEDIATE_RATIO:-0.5} \
		-e K6_LOAD_EXPECTED_MESSAGES=$${K6_LOAD_EXPECTED_MESSAGES:-} \
		--entrypoint bash $(K6_IMAGE) -lc 'k6 run --out opentelemetry /scripts/load.js 2>&1 | tee -a /data/lgtm/logs/k6-load.jsonl; exit $${PIPESTATUS[0]}'

## k6.test: Run k6 contract and load integration tests
k6.test: k6.contract k6.load

.PHONY: k6.build k6.contract k6.load k6.test
