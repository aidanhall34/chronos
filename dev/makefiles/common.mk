RUST_VERSION := $(shell grep 'channel' rust-toolchain.toml | sed 's/.*"\(.*\)"/\1/')

yellow := $(shell tput setaf 3 2>/dev/null || true)
normal := $(shell tput sgr0 2>/dev/null || true)

define pp
	@printf '$(yellow)$(1)$(normal)\n'
endef

define require_cmd
	@command -v $(1) >/dev/null 2>&1 || { \
		printf 'Missing required command: %s\n' '$(1)' >&2; \
		printf 'Install it with your system package manager, then run make setup again.\n' >&2; \
		exit 1; \
	}
endef
