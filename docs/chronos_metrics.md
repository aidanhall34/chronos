# Chronos Metrics

Generated from `dev/weaver/production/registry/chronos/metrics.yaml` by OpenTelemetry Weaver.

| Metric | Prometheus Name | Instrument | Unit | Attributes | Description |
| --- | --- | --- | --- | --- | --- |
| `chronos.message.consume.duration` | `chronos_message_consume_duration` | `histogram` | `s` | `chronos.consume.status`, `chronos.destination` | Duration of handle_message() in message_receiver. |
| `chronos.message.jitter` | `chronos_message_jitter` | `histogram` | `s` | - | Difference between actual publish time and client-requested deadline. |
| `chronos.message.process.duration` | `chronos_message_process_duration` | `histogram` | `s` | `chronos.process.status`, `chronos.processor.returned` | Duration of processor_message_ready() loop in message_processor. |
| `chronos.message.reset` | `chronos_message_reset` | `counter` | `{message}` | - | Number of records reset by reset_to_init_db() in the monitor task. |
| `chronos.message.wait.duration` | `chronos_message_wait_duration` | `histogram` | `s` | - | Time a message spent in the Kafka input queue before processing. |
