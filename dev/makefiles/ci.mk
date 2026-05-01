GITHUB_CONFIG ?= .github/config.json

## repo.config.apply: Apply GitHub repository and branch settings from .github/config.json
repo.config.apply:
	$(call pp,apply GitHub repository config from $(GITHUB_CONFIG)...)
	scripts/apply-github-config.sh "$(GITHUB_CONFIG)"

.PHONY: repo.config.apply
