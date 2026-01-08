# Convenience Makefile for GnawTreeWriter — useful MCP workflow targets
#
# Usage examples:
#   make start           # start server in background (writes PID & log)
#   make serve           # run server in foreground (blocking)
#   make stop            # stop server
#   make client-list     # run client list
#   make analyze FILE=examples/foo.py
#   make test-mcp        # start server, run checks, stop server
#   make ci-check        # build & test with mcp feature
#
# Environment / overrideable variables:
#   ADDR     Default bind address (host:port)
#   URL      Full URL to server (e.g. http://127.0.0.1:8080/)
#   TOKEN    Bearer token
#   PIDFILE  Path to store server PID (when using background start)
#   LOGFILE  Path to server log
#   RELEASE  Set to true to run release builds for examples
#
# Examples:
#   make start ADDR=127.0.0.1:9000 TOKEN=secret
#   make analyze FILE=examples/foo.py URL=http://127.0.0.1:9000/ TOKEN=secret
#

# Defaults (override on the make command line)
ADDR ?= 127.0.0.1:8080
URL ?= http://127.0.0.1:8080/
TOKEN ?= secret
PIDFILE ?= .mcp-server.pid
LOGFILE ?= .mcp-server.log
RELEASE ?= false

.PHONY: help start serve stop status client-list client-init client-analyze call test-mcp install ci-check fmt clippy check clean

.DEFAULT_GOAL := help

help:
	@echo "Useful targets for MCP work"
	@echo "  make start                 # start server in background (writes PID & log)"
	@echo "  make serve                 # run server in foreground (blocking)"
	@echo "  make stop                  # stop server (reads PID file)"
	@echo "  make status                # query running server status"
	@echo "  make client-list           # run client list"
	@echo "  make client-init           # run client init"
	@echo "  make analyze FILE=<path>   # analyze file using the Rust example"
	@echo "  make call TOOL=<name> ARGS='<json>'  # run generic client call"
	@echo "  make test-mcp              # start server, run checks, stop server"
	@echo "  make install               # cargo install with mcp feature"
	@echo "  make ci-check              # build & test with mcp feature"
	@echo "  make fmt|clippy|check      # format / lint / check"
	@echo "  make clean                 # cargo clean"

# Start server in background (writes PID & log using scripts/mcp-serve.sh)
start:
	@echo "Starting MCP server (background): addr=$(ADDR) token=$(TOKEN)"
	./scripts/mcp-serve.sh --addr "$(ADDR)" --token "$(TOKEN)" --pid "$(PIDFILE)" --log "$(LOGFILE)"

# Run server in foreground (blocking, suitable for debugging)
serve:
	@echo "Running MCP server (foreground): addr=$(ADDR) token=$(TOKEN)"
	./scripts/mcp-serve.sh --addr "$(ADDR)" --token "$(TOKEN)" --foreground

# Stop server (reads PID file)
stop:
	@echo "Stopping MCP server (pidfile=$(PIDFILE))"
	./scripts/mcp-stop.sh --pid "$(PIDFILE)" --log "$(LOGFILE)"

# Query server status (uses gnawtreewriter binary if installed)
status:
	@echo "Querying MCP server at $(URL)"
	@gnawtreewriter mcp status --url "$(URL)" --token "$(TOKEN)" 2>/dev/null || \
		(cargo run --features mcp -- mcp status --url "$(URL)" --token "$(TOKEN)")

# Client convenience targets (wrappers around scripts/mcp-client.sh)
client-list:
	./scripts/mcp-client.sh --url "$(URL)" --token "$(TOKEN)" list

client-init:
	./scripts/mcp-client.sh --url "$(URL)" --token "$(TOKEN)" init

client-analyze:
	@if [ -z "$(FILE)" ]; then echo "Usage: make analyze FILE=<path>"; exit 1; fi
	./scripts/mcp-client.sh --url "$(URL)" --token "$(TOKEN)" analyze "$(FILE)"

call:
	@if [ -z "$(TOOL)" ]; then echo "Usage: make call TOOL=<tool> [ARGS='<json>']"; exit 1; fi
	./scripts/mcp-client.sh --url "$(URL)" --token "$(TOKEN)" call "$(TOOL)" "$(ARGS)"

# Full local end-to-end test (start -> list/init/analyse -> stop)
test-mcp:
	./scripts/test-mcp.sh --addr "$(ADDR)" --token "$(TOKEN)"

# Install the CLI with MCP feature enabled
install:
	cargo install --path . --features mcp

# CI helper: build & run tests with MCP feature
ci-check:
	cargo build --release --features mcp
	cargo test --features mcp --no-fail-fast

# Dev helpers
fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

check:
	cargo check --features mcp

clean:
	cargo clean
	@echo "Note: PID/log files are not removed by this command. Use 'make stop' to stop server and remove PID file."

# EOF
