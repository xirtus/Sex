ENTRYPOINT := ./scripts/entrypoint_build.sh

full:
	@$(ENTRYPOINT)

payload:
	@$(ENTRYPOINT)

ci:
	@$(ENTRYPOINT)

# Legacy targets retained only as explicit hard-fail stubs.
iso:
	@echo "[FAIL] direct iso target disabled. build is sealed to trace executor."
	@exit 1

run-sasos:
	@echo "[FAIL] direct run target disabled. build is sealed to trace executor."
	@exit 1

build-tools:
	@echo "[FAIL] direct tools target disabled. build is sealed to trace executor."
	@exit 1

# Legacy internal targets retained as hard-fail stubs.
iso-internal:
	@echo "[FAIL] legacy internal target disabled. use sealed trace executor."
	@exit 1

run-sasos-internal:
	@echo "[FAIL] legacy internal target disabled. use sealed trace executor."
	@exit 1

build-tools-internal:
	@echo "[FAIL] legacy internal target disabled. use sealed trace executor."
	@exit 1
