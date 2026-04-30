# Chronos Metrics

Generated from `examples/weaver/registry/chronos/metrics.yaml` by OpenTelemetry Weaver.

| Metric | Prometheus Name | Instrument | Unit | Attributes | Description |
| --- | --- | --- | --- | --- | --- |
| `messaging.client.operation.duration` | `messaging_client_operation_duration_seconds` | `histogram` | `s` | `messaging.destination.name`, `messaging.operation.name`, `messaging.system` | Duration of handle_message() in message_receiver. |
| `messaging.client.consumed.messages` | `messaging_client_consumed_messages_total` | `counter` | `{message}` | `messaging.destination.name`, `messaging.operation.name`, `messaging.system` | Total number of Chronos input messages consumed. |
| `chronos.message.jitter` | `chronos_message_jitter_seconds` | `histogram` | `s` | - | Difference between actual publish time and client-requested deadline. |
| `messaging.process.duration` | `messaging_process_duration_seconds` | `histogram` | `s` | `messaging.destination.name`, `messaging.operation.name`, `messaging.system` | Duration of processor_message_ready() loop in message_processor. |
| `chronos.message.reset` | `chronos_messages_reset_total` | `counter` | `{message}` | - | Number of records reset by reset_to_init_db() in the monitor task. |
| `chronos.message.wait.duration` | `chronos_message_wait_duration_seconds` | `histogram` | `s` | - | Time a message spent in the Kafka input queue before processing. |
