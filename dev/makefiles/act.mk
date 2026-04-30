#!make
SHELL := /bin/bash

ACT_EVENT ?= push
ACT_JOB ?= pre-commit
ACT_RUNNER_IMAGE ?= catthehacker/ubuntu:act-latest
ACT_ARTIFACT_DIR ?= /tmp/chronos-act-artifacts
ACT_EVENT_DIR ?= /tmp/chronos-act-events
ACT_FLAGS ?= -P ubuntu-latest=$(ACT_RUNNER_IMAGE) --artifact-server-path $(ACT_ARTIFACT_DIR)

CI_WORKFLOW ?= .github/workflows/CI.yaml
PRE_COMMIT_WORKFLOW ?= .github/workflows/pre-commit.yml
TEST_WORKFLOW ?= .github/workflows/test.yml
SCAN_WORKFLOW ?= .github/workflows/scan.yml
BUILD_BINARY_WORKFLOW ?= .github/workflows/build-binary.yml
BUILD_CONTAINER_WORKFLOW ?= .github/workflows/build-container.yml
SBOM_WORKFLOW ?= .github/workflows/sbom.yml

SBOM_TARGET_TYPE ?= release
SBOM_TARGET_REF ?= .

.PHONY: act.ci act.ci.job act.pre-commit act.test act.scan act.build-binary act.build-container act.sbom act.sbom.container act.sbom.release

act.ci:
	mkdir -p "$(ACT_ARTIFACT_DIR)"
	act push -W "$(CI_WORKFLOW)" $(ACT_FLAGS)

act.ci.job:
	mkdir -p "$(ACT_ARTIFACT_DIR)"
	act push -W "$(CI_WORKFLOW)" -j "$(ACT_JOB)" $(ACT_FLAGS)

act.pre-commit:
	act workflow_dispatch -W "$(PRE_COMMIT_WORKFLOW)" $(ACT_FLAGS)

act.test:
	act workflow_dispatch -W "$(TEST_WORKFLOW)" $(ACT_FLAGS)

act.scan:
	act workflow_dispatch -W "$(SCAN_WORKFLOW)" $(ACT_FLAGS)

act.build-binary:
	mkdir -p "$(ACT_ARTIFACT_DIR)"
	act workflow_dispatch -W "$(BUILD_BINARY_WORKFLOW)" $(ACT_FLAGS)

act.build-container:
	act workflow_dispatch -W "$(BUILD_CONTAINER_WORKFLOW)" $(ACT_FLAGS)

act.sbom:
	mkdir -p "$(ACT_ARTIFACT_DIR)" "$(ACT_EVENT_DIR)"
	printf '{"inputs":{"target-type":"%s","target-ref":"%s"}}\n' "$(SBOM_TARGET_TYPE)" "$(SBOM_TARGET_REF)" > "$(ACT_EVENT_DIR)/sbom.json"
	act workflow_dispatch -W "$(SBOM_WORKFLOW)" -e "$(ACT_EVENT_DIR)/sbom.json" $(ACT_FLAGS)

act.sbom.container:
	$(MAKE) -f dev/makefiles/act.mk act.sbom SBOM_TARGET_TYPE=container

act.sbom.release:
	$(MAKE) -f dev/makefiles/act.mk act.sbom SBOM_TARGET_TYPE=release
