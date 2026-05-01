## integration: Start deps, migrate, run Chronos, publish test message, verify metrics
integration: build
	$(call pp,running integration test...)
	@bash scripts/integration.sh

## integration.down: Stop docker services started by make integration
integration.down:
	$(call pp,stopping integration services...)
	docker compose --project-name chronos -f dev/docker-compose/compose.yaml stop postgres kafka 2>/dev/null || true
	docker compose --project-name chronos -f dev/docker-compose/compose.yaml rm -f postgres kafka 2>/dev/null || true

.PHONY: integration integration.down
