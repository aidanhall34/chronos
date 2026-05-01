SHELL := /usr/bin/env bash
.DEFAULT_GOAL := help

MAKEFILES_DIR := dev/makefiles
COMMON_MAKEFILE := $(MAKEFILES_DIR)/common.mk
MAKEFILE_PARTS := $(filter-out $(COMMON_MAKEFILE),$(sort $(wildcard $(MAKEFILES_DIR)/*.mk)))

include $(COMMON_MAKEFILE)
include $(MAKEFILE_PARTS)

## help: Print available make targets
help:
	@echo "Choose a command to run:"
	@awk '/^## / { help=substr($$0, 4); sub(/^[^:]+: /, "", help); next } /^[A-Za-z0-9_.-]+:/ { if (help != "") { split($$0, target, ":"); printf "  %-28s %s\n", target[1], help; help="" } }' Makefile $(COMMON_MAKEFILE) $(MAKEFILE_PARTS) | sort

.PHONY: help
