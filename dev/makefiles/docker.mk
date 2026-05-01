COMPOSE_PROJECT_NAME ?= chronos
COMPOSE_FILE_BASE := dev/docker-compose/compose.yaml
COMPOSE_FILE_JAEGER := dev/docker-compose/jaeger.yaml
COMPOSE_FILE_LGTM := dev/docker-compose/lgtm.yaml
BACKEND_ARG := $(firstword $(filter jaeger lgtm,$(MAKECMDGOALS)))
BACKEND ?= $(if $(BACKEND_ARG),$(BACKEND_ARG),jaeger)
COMPOSE_BACKEND_FILE := $(if $(filter lgtm,$(BACKEND)),$(COMPOSE_FILE_LGTM),$(COMPOSE_FILE_JAEGER))
DOCKER_COMPOSE := docker compose --project-name $(COMPOSE_PROJECT_NAME) -f $(COMPOSE_FILE_BASE) -f $(COMPOSE_BACKEND_FILE)
DOCKER_COMPOSE_JAEGER := docker compose --project-name $(COMPOSE_PROJECT_NAME) -f $(COMPOSE_FILE_BASE) -f $(COMPOSE_FILE_JAEGER)
DOCKER_COMPOSE_LGTM := docker compose --project-name $(COMPOSE_PROJECT_NAME) -f $(COMPOSE_FILE_BASE) -f $(COMPOSE_FILE_LGTM)

## up: Build and start Chronos, dependencies, and observability. Use make up lgtm or BACKEND=lgtm for LGTM
up:
	$(call pp,starting docker compose stack with $(BACKEND) observability...)
	$(DOCKER_COMPOSE) up -d --build

## down: Stop the docker compose stack
down:
	$(call pp,stopping docker compose stack...)
	$(DOCKER_COMPOSE_LGTM) down 2>/dev/null || true
	$(DOCKER_COMPOSE_JAEGER) down 2>/dev/null || true

## docker.config: Render the docker compose configuration
docker.config:
	$(DOCKER_COMPOSE) config

## docker.up: Legacy alias for make up
docker.up: up

## docker.down: Legacy alias for make down
docker.down: down

jaeger lgtm:
	@:

.PHONY: up down docker.config docker.up docker.down jaeger lgtm
