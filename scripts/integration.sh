#!/usr/bin/env bash
# integration.sh — starts Docker dependencies, runs migrations, starts Chronos
# locally, publishes a test message, and verifies metrics are being recorded.
#
# Usage: called by `make integration` from the repo root.
set -euo pipefail

# ─── configuration ────────────────────────────────────────────────────────────
KAFKA_EXT_PORT="${KAFKA_EXT_PORT:-9094}"
PG_PORT="${PG_PORT:-5432}"
METRICS_PORT="${OTEL_EXPORTER_PROMETHEUS_PORT:-${METRICS_PORT:-9090}}"
CHRONOS_PID_FILE="/tmp/chronos_integration.pid"
CHRONOS_LOG="/tmp/chronos_integration.log"
MAX_WAIT=120   # seconds to wait for each readiness check

# Unique ID for this test run — used to identify our message on the output topic
MSG_ID="integration-test-$(date +%s)"

# ─── helpers ──────────────────────────────────────────────────────────────────
log()  { printf '\033[0;33m%s\033[0m\n' "→ $*"; }
ok()   { printf '\033[0;32m%s\033[0m\n' "✓ $*"; }
fail() { printf '\033[0;31m%s\033[0m\n' "✗ $*" >&2; exit 1; }

wait_for() {
    local label="$1"; shift
    local elapsed=0
    printf '%s ' "→ Waiting for ${label}..."
    until "$@" > /dev/null 2>&1; do
        printf '.'
        sleep 2
        elapsed=$((elapsed + 2))
        if [ "${elapsed}" -ge "${MAX_WAIT}" ]; then
            echo ""
            fail "Timed out waiting for ${label} after ${MAX_WAIT}s"
        fi
    done
    echo " ready"
}

# cleanup() {
#     if [ -f "${CHRONOS_PID_FILE}" ]; then
#         local pid
#         pid="$(cat "${CHRONOS_PID_FILE}")"
#         if kill -0 "${pid}" 2>/dev/null; then
#             log "Stopping Chronos (pid ${pid})..."
#             kill "${pid}" 2>/dev/null || true
#             wait "${pid}" 2>/dev/null || true
#         fi
#         rm -f "${CHRONOS_PID_FILE}"
#     fi
# }
# trap cleanup EXIT

# ─── 1. start infrastructure ──────────────────────────────────────────────────
log "Starting infrastructure (postgres + kafka)..."
docker compose up -d postgres kafka

# ─── 2. wait for postgres ─────────────────────────────────────────────────────
wait_for "postgres" \
    docker compose exec -T postgres pg_isready -U admin -d chronos_db

# ─── 3. wait for kafka ────────────────────────────────────────────────────────
wait_for "kafka" \
    docker compose exec -T kafka \
        /opt/bitnami/kafka/bin/kafka-topics.sh --bootstrap-server localhost:9092 --list

# ─── 4. run migrations ────────────────────────────────────────────────────────
log "Running database migrations..."
PG_HOST=localhost \
PG_PORT="${PG_PORT}" \
PG_USER=admin \
PG_PASSWORD=admin \
PG_DATABASE=chronos_db \
    cargo run --quiet --package pg_mig --bin chronos-pg-migrations
ok "Migrations complete"

# ─── 5. start chronos in background ──────────────────────────────────────────
log "Starting Chronos (logs → ${CHRONOS_LOG})..."
KAFKA_HOST=localhost \
KAFKA_PORT="${KAFKA_EXT_PORT}" \
KAFKA_CLIENT_ID=chronos \
KAFKA_GROUP_ID=chronos \
KAFKA_IN_TOPIC=chronos.in \
KAFKA_OUT_TOPIC=chronos.out \
KAFKA_USERNAME="" \
KAFKA_PASSWORD="" \
PG_HOST=localhost \
PG_PORT="${PG_PORT}" \
PG_USER=admin \
PG_PASSWORD=admin \
PG_DATABASE=chronos_db \
PG_POOL_SIZE=10 \
RUST_LOG=warn \
OTEL_EXPORTER_PROMETHEUS_HOST=0.0.0.0 \
OTEL_EXPORTER_PROMETHEUS_PORT="${METRICS_PORT}" \
MONITOR_DB_POLL=5 \
PROCESSOR_DB_POLL=5 \
TIMING_ADVANCE=0 \
FAIL_DETECT_INTERVAL=10 \
    cargo run --quiet --package chronos_bin --bin chronos \
        > "${CHRONOS_LOG}" 2>&1 &
echo $! > "${CHRONOS_PID_FILE}"

# ─── 6. wait for metrics endpoint ────────────────────────────────────────────
wait_for "Chronos metrics endpoint" \
    curl -sf "http://localhost:${METRICS_PORT}/metrics"

# ─── 7. publish test message ─────────────────────────────────────────────────
# The deadline is 1 minute in the past so Chronos fires the message immediately
# to the output topic, exercising the full consume → store-or-fire path.
log "Publishing test message (id: ${MSG_ID})..."
CHRONOS_MSG_ID="${MSG_ID}" \
KAFKA_HOST=localhost \
KAFKA_PORT="${KAFKA_EXT_PORT}" \
KAFKA_CLIENT_ID=chronos-test-publisher \
KAFKA_GROUP_ID=chronos-test-publisher \
KAFKA_IN_TOPIC=chronos.in \
KAFKA_OUT_TOPIC=chronos.out \
KAFKA_USERNAME="" \
KAFKA_PASSWORD="" \
    cargo run --quiet --package chronos_ex --example publish_test_message
ok "Message published"

# ─── 8. verify message fired to output topic ─────────────────────────────────
# Consume from chronos.out from the beginning, waiting up to 30s for the message
# to appear. kafka-console-consumer exits when max-messages is reached OR when
# no new messages arrive within timeout-ms — whichever comes first.
# The || true prevents set -e from aborting on the consumer's non-zero exit
# (timeout reached) which is normal when the topic drains before max-messages.
log "Waiting for message ${MSG_ID} on chronos.out (up to 30s)..."
FIRED_OUTPUT=$(
    docker compose exec -T kafka \
        /opt/bitnami/kafka/bin/kafka-console-consumer.sh \
        --bootstrap-server localhost:9092 \
        --topic chronos.out \
        --from-beginning \
        --max-messages 50 \
        --timeout-ms 30000 \
        2>/dev/null || true
)

if echo "${FIRED_OUTPUT}" | grep -q "${MSG_ID}"; then
    ok "Message ${MSG_ID} arrived on chronos.out"
else
    echo ""
    printf '\033[0;31m%s\033[0m\n' "✗ Message ${MSG_ID} was NOT found on chronos.out" >&2
    echo "  Last 20 lines of Chronos log:" >&2
    tail -20 "${CHRONOS_LOG}" >&2
    fail "Message delivery test failed"
fi

# ─── 9. show metrics ─────────────────────────────────────────────────────────
echo ""
echo "══════════════════════════════════════════════════════"
echo "  Chronos metrics  (http://localhost:${METRICS_PORT}/metrics)"
echo "══════════════════════════════════════════════════════"
curl -sf "http://localhost:${METRICS_PORT}/metrics" \
    | grep -E "^(# HELP|# TYPE|msg_)" \
    | sort
echo ""

# ─── 10. verify all five metric families are present ─────────────────────────
log "Verifying metric families..."
METRICS_OUTPUT="$(curl -sf "http://localhost:${METRICS_PORT}/metrics")"
EXPECTED_METRICS=(
    "msg_consume_latency"
    "msg_process_latency"
    "msg_wait_time"
    "msg_jitter"
    "msg_reset"
)
ALL_OK=true
for metric in "${EXPECTED_METRICS[@]}"; do
    if echo "${METRICS_OUTPUT}" | grep -q "^# HELP ${metric}"; then
        ok "${metric} present"
    else
        printf '\033[0;31m%s\033[0m\n' "✗ ${metric} MISSING" >&2
        ALL_OK=false
    fi
done

echo ""
if [ "${ALL_OK}" = "true" ]; then
    ok "All metrics verified"
else
    fail "One or more metrics are missing — check ${CHRONOS_LOG}"
fi

echo ""
ok "Integration test complete"
echo "  Chronos logs: ${CHRONOS_LOG}"
echo "  Run 'make integration.down' to stop Docker services."
