SIM = target/release/cache-simulator
CARGO = cargo

ifeq ($(mode), debug)
	__MODE = --debug
else
	__MODE =
endif

ifeq ($(config_path), )
	$(error config_path is empty)
endif

ifeq ($(trace_path), )
	$(error trace_path is empty)
endif

run:
	@echo "----------Build Simulator----------"
	@$(CARGO) build --release
	@echo "----------Start Simulation----------"
	@$(SIM) $(DEBUG) --config $(config_path) --trace $(trace_path)

clean:
	@$(CARGO) clean

.PHONY: clean all