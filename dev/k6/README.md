# Chronos k6 Integration Tests

The k6 image is built with `xk6-kafka` so tests can publish to and consume from the Chronos Kafka topics. k6 run metrics are exported with the built-in OpenTelemetry output.

## Targets

- `make k6.build` builds the custom k6 image.
- `make k6.contract` runs one pass through the important Chronos processing paths.
- `make k6.load` runs a constant-arrival-rate producer load test. The default profile tops out at 100 messages/sec.
- `K6_FULL_LOAD=true make k6.load` runs the full load profile at 1,000 messages/sec for one minute.
- `make k6.test` runs contract and load tests.

The full load profile is a production-scale signal, not a guaranteed local-dev pass. It depends on k6 producer speed, k6 consumer drain speed, Docker host capacity, Kafka throughput, PostgreSQL throughput, and Chronos capacity. It may require production-like infrastructure to satisfy the 1,000 messages/sec throughput target and the 500 ms p99.9 observed scheduling jitter threshold.

The load test records `chronos_scheduling_jitter` from the Kafka output record timestamp minus the requested scheduled timestamp. It does not use the time k6 consumes or drains the output topic.

By default the recipes use the LGTM compose network and send k6 OTLP metrics to `lgtm:4317`. In GitHub Actions outside `act`, set `K6_CI_OTLP_ENDPOINT`; the default is `host.docker.internal:4317`. When running under `act`, the recipes keep using the LGTM container.

Logs from k6 are appended to `dev/lgtm/runtime-logs/*.jsonl`, which is mounted into the LGTM collector filelog receiver.
