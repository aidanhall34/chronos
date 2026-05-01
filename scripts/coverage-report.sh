#!/usr/bin/env sh

set -eu

if ! cargo llvm-cov --version >/dev/null 2>&1; then
	printf 'cargo-llvm-cov not installed; writing raw LLVM coverage profiles under target/coverage.\n' >&2
	coverage_dir="$(pwd)/target/coverage"
	mkdir -p "${coverage_dir}"
	CARGO_INCREMENTAL=0 \
		CARGO_HUSKY_DONT_INSTALL_HOOKS=true \
		RUSTFLAGS="${RUSTFLAGS:-} -Cinstrument-coverage" \
		LLVM_PROFILE_FILE="${coverage_dir}/chronos-%p-%m.profraw" \
		cargo test
	exit 0
fi

cargo llvm-cov --workspace --all-targets
