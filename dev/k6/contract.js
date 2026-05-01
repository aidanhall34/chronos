import { check, sleep } from "k6";
import encoding from "k6/encoding";
import { Counter } from "k6/metrics";
import { Producer, Consumer } from "k6/x/kafka";

const brokers = (__ENV.KAFKA_BROKERS || "kafka:9092").split(",");
const inputTopic = __ENV.KAFKA_IN_TOPIC || "chronos.in";
const outputTopic = __ENV.KAFKA_OUT_TOPIC || "chronos.out";
const runId = __ENV.K6_RUN_ID || `contract-${Date.now()}`;
const outputTimeoutMs = Number(__ENV.K6_CONTRACT_OUTPUT_TIMEOUT_MS || 15000);

const exercisedPaths = new Counter("chronos_contract_paths_exercised");

export const options = {
  scenarios: {
    contract: {
      executor: "shared-iterations",
      vus: 1,
      iterations: 1,
      maxDuration: "30s",
    },
  },
  thresholds: {
    checks: ["rate==1"],
    chronos_contract_paths_exercised: ["count>=4"],
  },
};

const producer = new Producer({
  brokers,
  topic: inputTopic,
  autoCreateTopic: true,
  requiredAcks: 1,
});

const consumer = new Consumer({
  brokers,
  topic: outputTopic,
  groupId: `${runId}-out`,
  startOffset: "start_offsets_first_offset",
  maxWait: "500ms",
});

function deadline(offsetMs) {
  return new Date(Date.now() + offsetMs).toISOString();
}

function payload(id, extra = {}) {
  return JSON.stringify({
    source: "k6-contract",
    run_id: runId,
    message_id: id,
    sent_at_ms: Date.now(),
    ...extra,
  });
}

function chronosHeaders(id, deadlineValue) {
  return {
    chronosMessageId: id,
    chronosDeadline: deadlineValue,
  };
}

function bytesToString(value) {
  if (typeof value === "string") {
    return value;
  }
  return String.fromCharCode.apply(null, Array.from(value || []));
}

function produceMessage({ id, key = id, value = payload(id), deadlineMs = -1000, headers = null }) {
  const message = {
    value: encoding.b64encode(value),
    headers: headers || chronosHeaders(id, deadline(deadlineMs)),
  };
  if (key !== null) {
    message.key = encoding.b64encode(key);
  }
  producer.produce({ messages: [message] });
}

function consumeUntil(id, timeoutMs) {
  const expiresAt = Date.now() + timeoutMs;
  while (Date.now() < expiresAt) {
    const messages = consumer.consume({ maxMessages: 25, expectTimeout: true });
    for (const message of messages) {
      const value = bytesToString(message.value);
      if (value.includes(id)) {
        return value;
      }
    }
    sleep(0.1);
  }
  return "";
}

export default function () {
  const immediatePassId = `${runId}-immediate-pass`;
  produceMessage({ id: immediatePassId, deadlineMs: -1000 });
  const immediateOutput = consumeUntil(immediatePassId, outputTimeoutMs);
  check(immediateOutput, {
    "immediate kafka path publishes output": (value) => value.includes(immediatePassId),
  });
  exercisedPaths.add(1, { chronos_destination: "kafka", chronos_status: "pass" });

  const delayedPassId = `${runId}-delayed-pass`;
  produceMessage({ id: delayedPassId, deadlineMs: 750 });
  const delayedOutput = consumeUntil(delayedPassId, outputTimeoutMs);
  check(delayedOutput, {
    "postgres delay path publishes output": (value) => value.includes(delayedPassId),
  });
  exercisedPaths.add(1, { chronos_destination: "postgres", chronos_status: "pass" });

  const postgresFailId = `${runId}-postgres-fail`;
  produceMessage({ id: postgresFailId, value: "not-json", deadlineMs: 60_000 });
  sleep(1);
  const postgresFailOutput = consumeUntil(postgresFailId, 1000);
  check(postgresFailOutput, {
    "invalid future payload is not published": (value) => value === "",
  });
  exercisedPaths.add(1, { chronos_destination: "postgres", chronos_status: "fail" });

  const kafkaFailId = `${runId}-kafka-fail`;
  produceMessage({ id: kafkaFailId, key: null, deadlineMs: -1000 });
  sleep(1);
  const kafkaFailOutput = consumeUntil(kafkaFailId, 1000);
  check(kafkaFailOutput, {
    "missing key immediate payload is not published": (value) => value === "",
  });
  exercisedPaths.add(1, { chronos_destination: "kafka", chronos_status: "fail" });

  sleep(1);
}

export function teardown() {
  producer.close();
  consumer.close();
}
