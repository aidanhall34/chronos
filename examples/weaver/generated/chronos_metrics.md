# Chronos Metrics

Generated from `examples/weaver/registry/chronos/metrics.yaml` by OpenTelemetry Weaver.

| Metric | Prometheus Name | Instrument | Unit | Attributes | Description |
| --- | --- | --- | --- | --- | --- |
| `chronos.message.consume.duration` | `msg_consume_latency` | `histogram` | `s` | `destination`, `status` | Duration of handle_message() in message_receiver. |
| `chronos.message.jitter` | `msg_jitter` | `histogram` | `s` | - | Difference between actual publish time and client-requested deadline. |
| `chronos.message.process.duration` | `msg_process_latency` | `histogram` | `s` | `returned`, `status` | Duration of processor_message_ready() loop in message_processor. |
| `chronos.message.reset` | `msg_reset` | `counter` | `{message}` | - | Number of records reset by reset_to_init_db() in the monitor task. |
| `chronos.message.wait.duration` | `msg_wait_time` | `histogram` | `s` | - | Time a message spent in the Kafka input queue before processing. |
