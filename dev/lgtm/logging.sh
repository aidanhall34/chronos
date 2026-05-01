#!/usr/bin/env bash

set -euo pipefail

function json_lines() {
	service=$1
	stream=$2

	awk -v service="${service}" -v stream="${stream}" '
		function escape_json(value) {
			gsub(/\\/, "\\\\", value)
			gsub(/"/, "\\\"", value)
			gsub(/\t/, "\\t", value)
			gsub(/\r/, "\\r", value)
			return value
		}
		/^[[:space:]]*\{/ {
			print
			fflush()
			next
		}
		{
			message = escape_json($0)
			printf("{\"service\":\"%s\",\"stream\":\"%s\",\"message\":\"%s\"}\n", service, stream, message)
			fflush()
		}
	'
}

function run_with_logging() {
	name=$1
	shift
	envvar=$1
	shift

	case "${name}" in
	"OpenTelemetry Collector"*) service_name=otelcol ;;
	*) service_name=${name%% *} ;;
	esac
	safe_name=$(printf '%s' "${service_name}" | tr '[:upper:]' '[:lower:]' | tr -cd '[:alnum:]_.-')
	log_dir="${LGTM_LOG_DIR:-/data/lgtm/logs}"
	log_file="${log_dir}/${safe_name}.jsonl"

	if [[ ${envvar} == "true" || ${ENABLE_LOGS_ALL:-false} == "true" ]]; then
		echo "Running ${name} logging=true file=${log_file}"
		mkdir -p "${log_dir}"
		exec "$@" > >(json_lines "${name}" stdout | tee -a "${log_file}") 2> >(json_lines "${name}" stderr | tee -a "${log_file}" >&2)
	else
		echo "Running ${name} logging=false"
		exec "$@" >/dev/null 2>&1
	fi
}
