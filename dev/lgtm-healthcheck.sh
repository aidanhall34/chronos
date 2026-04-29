#!/usr/bin/env sh

set -eu

check_service() {
	name=$1
	url=$2

	echo "Checking ${name} at ${url}"
	curl -sf "${url}" >/dev/null
}

check_service "Grafana" "http://127.0.0.1:3000/api/health"
check_service "Loki" "http://127.0.0.1:3100/ready"
check_service "Tempo" "http://127.0.0.1:3200/ready"
check_service "Pyroscope" "http://127.0.0.1:4040/ready"
check_service "Prometheus" "http://127.0.0.1:9090/-/ready"
check_service "OpenTelemetry Collector" "http://127.0.0.1:13133/ready"

echo "All LGTM services healthy"
