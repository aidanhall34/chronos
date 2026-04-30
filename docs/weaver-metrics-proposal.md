# Managing Chronos Metrics with OpenTelemetry Weaver

This proposal keeps the model from `examples/prom_otlp_mock.rs`: metric definitions live once, then the Prometheus and OTLP backends register instruments from that shared definition set. Weaver becomes the source of truth for the shared definition set.

## Example Spec

The example registry is in `examples/weaver/registry/chronos/metrics.yaml`. It defines Chronos metrics using OpenTelemetry-style names:

| OpenTelemetry metric | Prometheus output name | Instrument |
| --- | --- | --- |
| `chronos.message.consumed` | `chronos_messages_consumed_total` | counter |
| `chronos.message.consume.duration` | `chronos_message_consume_duration_seconds` | histogram |
| `chronos.message.process.duration` | `chronos_message_process_duration_seconds` | histogram |
| `chronos.message.wait.duration` | `chronos_message_wait_duration_seconds` | histogram |
| `chronos.message.jitter` | `chronos_message_jitter_seconds` | histogram |
| `chronos.message.reset` | `chronos_messages_reset_total` | counter |

The checked-in generated example is `examples/weaver/generated/chronos_metric_definitions.rs`. It mirrors the `MetricDefinition` table in `examples/prom_otlp_mock.rs`, with both `otel_name` and `prometheus_name` so each exporter can use the native naming convention it expects.

## Suggested Workflow

Pin Weaver to the version used by the branch and make the generated file reproducible:

```sh
WEAVER_VERSION=0.23.0
docker run --rm \
  -v "$(pwd):/work" \
  -w /work \
  "otel/weaver:v${WEAVER_VERSION}" \
  registry check -r examples/weaver/registry
docker run --rm \
  -v "$(pwd):/work" \
  -w /work \
  "otel/weaver:v${WEAVER_VERSION}" \
  registry generate \
  -r examples/weaver/registry \
  --templates examples/weaver/templates \
  rust chronos_bin/src/metrics/generated
rustfmt chronos_bin/src/metrics/generated/chronos_metric_definitions.rs
```

Add a `make metrics.generate` target for the `registry generate` command and a `make metrics.check` target that runs `weaver registry check` plus a diff check that generated files are current. The pre-commit script can then call `make metrics.check` once Weaver is a documented development dependency.

## Implementation Path

1. Keep the current Prometheus registry working while introducing generated definitions behind a small module such as `chronos_bin/src/metrics/generated/definitions.rs`.
2. Replace the hand-written metric creation in `chronos_bin/src/metrics/registry.rs` with a loop over generated `METRIC_DEFINITIONS`, following the backend loop already sketched in `examples/prom_otlp_mock.rs`.
3. Preserve compatibility temporarily by either exporting the current `msg_*` Prometheus names or by dual-registering old and new names for one release. The example spec prefers OpenTelemetry names and Prometheus-conventional rendered names.
4. Use generated attribute constants for label names so call sites record attributes by typed identifiers instead of string literals.
5. After the generated table is in use, add a test that gathers the registry and asserts every generated `prometheus_name` appears in the text output.

Weaver can generate all of the static definition layer: metric IDs, names, descriptions, units, label names, bucket boundaries, and eventually attribute constants. Runtime behavior should remain hand-written because it contains Chronos-specific decisions: which events record which metric, pre-warming label combinations, exporter selection, and shutdown behavior.
