import { check, sleep } from "k6";
import encoding from "k6/encoding";
import { Counter, Trend } from "k6/metrics";
import { Producer, Consumer } from "k6/x/kafka";

const brokers = (__ENV.KAFKA_BROKERS || "kafka:9092").split(",");
const inputTopic = __ENV.KAFKA_IN_TOPIC || "chronos.in";
const outputTopic = __ENV.KAFKA_OUT_TOPIC || "chronos.out";
const rate = Number(__ENV.K6_LOAD_RATE || 100);
const duration = __ENV.K6_LOAD_DURATION || "1m";
const consumeDuration = __ENV.K6_LOAD_CONSUME_DURATION || "90s";
const delayedScheduleDelayMs = Number(__ENV.K6_LOAD_DELAY_MS || 1000);
const immediateScheduleDelayMs = Number(__ENV.K6_LOAD_IMMEDIATE_DELAY_MS || -1000);
const immediateRatio = clampRatio(Number(__ENV.K6_LOAD_IMMEDIATE_RATIO || 0.5));
const runId = __ENV.K6_RUN_ID || `load-${Date.now()}`;
const expectedMessages = Number(__ENV.K6_LOAD_EXPECTED_MESSAGES || Math.floor(rate * durationSeconds(duration)));

const published = new Counter("chronos_messages_published");
const consumed = new Counter("chronos_messages_consumed");
const timestampErrors = new Counter("chronos_output_timestamp_errors");
const schedulingJitter = new Trend("chronos_scheduling_jitter", true);
const immediateOutputDelay = new Trend("chronos_immediate_output_delay", true);

export const options = {
  scenarios: {
    queueing_load: {
      executor: "constant-arrival-rate",
      exec: "produceInput",
      rate,
      timeUnit: "1s",
      duration,
      preAllocatedVUs: Number(__ENV.K6_LOAD_PREALLOCATED_VUS || 100),
      maxVUs: Number(__ENV.K6_LOAD_MAX_VUS || 500),
    },
    output_drain: {
      executor: "constant-vus",
      exec: "consumeOutput",
      vus: 1,
      duration: consumeDuration,
      gracefulStop: "5s",
    },
  },
  summaryTrendStats: ["avg", "min", "med", "p(95)", "p(99)", "p(99.9)", "max"],
  thresholds: {
    checks: ["rate>=0.999"],
    dropped_iterations: ["count==0"],
    chronos_messages_published: [`count>=${expectedMessages}`],
    chronos_messages_consumed: [`count>=${expectedMessages}`],
    chronos_output_timestamp_errors: ["count==0"],
    chronos_scheduling_jitter: ["p(99.9)<500"],
  },
};

let producer;
let consumer;
const seen = {};

function getProducer() {
  if (!producer) {
    producer = new Producer({
      brokers,
      topic: inputTopic,
      autoCreateTopic: true,
      requiredAcks: 1,
    });
  }
  return producer;
}

function getConsumer(data) {
  if (!consumer) {
    consumer = new Consumer({
      brokers,
      topic: outputTopic,
      groupId: `${data.runId}-out`,
      startOffset: "start_offsets_first_offset",
      maxWait: "500ms",
    });
  }
  return consumer;
}

function durationSeconds(value) {
  const match = String(value).match(/^(\d+)(ms|s|m|h)$/);
  if (!match) {
    return 60;
  }
  const amount = Number(match[1]);
  switch (match[2]) {
    case "ms":
      return amount / 1000;
    case "s":
      return amount;
    case "m":
      return amount * 60;
    case "h":
      return amount * 3600;
    default:
      return 60;
  }
}

function clampRatio(value) {
  if (Number.isNaN(value)) {
    return 0.5;
  }
  return Math.min(1, Math.max(0, value));
}

function shouldPublishImmediate() {
  return ((__ITER % 100) / 100) < immediateRatio;
}

function bytesToString(value) {
  if (typeof value === "string") {
    return value;
  }
  return String.fromCharCode.apply(null, Array.from(value || []));
}

export function setup() {
  return { runId, expectedMessages };
}

export function produceInput(data) {
  const publishedAtMs = Date.now();
  const id = `${data.runId}-${__VU}-${__ITER}-${publishedAtMs}`;
  const chronosPath = shouldPublishImmediate() ? "immediate" : "delayed";
  const scheduleDelayMs = chronosPath === "immediate" ? immediateScheduleDelayMs : delayedScheduleDelayMs;
  const scheduledAtMs = publishedAtMs + scheduleDelayMs;
  const message = {
    key: encoding.b64encode(id),
    value: encoding.b64encode(JSON.stringify({
      source: "k6-load",
      run_id: data.runId,
      message_id: id,
      chronos_path: chronosPath,
      published_at_ms: publishedAtMs,
      scheduled_at_ms: scheduledAtMs,
    })),
    headers: {
      chronosMessageId: id,
      chronosDeadline: new Date(scheduledAtMs).toISOString(),
    },
  };
  getProducer().produce({ messages: [message] });
  published.add(1, { chronos_path: chronosPath });
}

export function consumeOutput(data) {
  const messages = getConsumer(data).consume({ maxMessages: 500, expectTimeout: true });
  let matched = 0;
  for (const message of messages) {
    const value = bytesToString(message.value);
    if (!value.includes(data.runId)) {
      continue;
    }
    const parsed = JSON.parse(value);
    if (seen[parsed.message_id]) {
      continue;
    }
    seen[parsed.message_id] = true;
    const outputPublishedAtMs = Date.parse(message.time);
    if (Number.isNaN(outputPublishedAtMs)) {
      timestampErrors.add(1);
      continue;
    }
    consumed.add(1, { chronos_path: parsed.chronos_path || "unknown" });
    if (parsed.chronos_path === "delayed") {
      const scheduledAtMs = Number(parsed.scheduled_at_ms);
      if (Number.isNaN(scheduledAtMs)) {
        timestampErrors.add(1);
        continue;
      }
      schedulingJitter.add(Math.max(0, outputPublishedAtMs - scheduledAtMs), { chronos_path: "delayed" });
    } else {
      const publishedAtMs = Number(parsed.published_at_ms);
      if (Number.isNaN(publishedAtMs)) {
        timestampErrors.add(1);
        continue;
      }
      immediateOutputDelay.add(Math.max(0, outputPublishedAtMs - publishedAtMs), { chronos_path: "immediate" });
    }
    matched += 1;
  }
  if (matched === 0) {
    sleep(0.1);
  }
}

export function teardown() {
  if (producer) {
    producer.flush();
    producer.close();
  }
  if (consumer) {
    consumer.close();
  }
  check(true, {
    "load test completed": (value) => value === true,
  });
}
