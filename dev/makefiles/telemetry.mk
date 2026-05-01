LGTM_IMAGE ?= grafana/otel-lgtm:0.24.1

## lgtm.validate: Validate LGTM Prometheus and OpenTelemetry Collector configs
lgtm.validate:
	$(call pp,validate LGTM Prometheus config with $(LGTM_IMAGE)...)
	docker run --rm \
		-v "$(PWD)/dev/lgtm/prometheus.yaml:/otel-lgtm/prometheus.yaml:ro" \
		--entrypoint /otel-lgtm/prometheus/promtool \
		$(LGTM_IMAGE) check config /otel-lgtm/prometheus.yaml
	$(call pp,validate LGTM OpenTelemetry Collector config with $(LGTM_IMAGE)...)
	docker run --rm \
		-v "$(PWD)/dev/lgtm/otelcol-contrib.yaml:/otel-lgtm/otelcol-config.yaml:ro" \
		--entrypoint /otel-lgtm/otelcol-contrib/otelcol-contrib \
		$(LGTM_IMAGE) validate --config=file:/otel-lgtm/otelcol-config.yaml --feature-gates=service.profilesSupport

## lgtm.up: Start the docker compose stack with Grafana LGTM
lgtm.up:
	$(MAKE) up BACKEND=lgtm

## lgtm.down: Stop the docker compose stack with Grafana LGTM
lgtm.down:
	$(MAKE) down BACKEND=lgtm

.PHONY: lgtm.validate lgtm.up lgtm.down
