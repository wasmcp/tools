.PHONY: all setup build clean publish help list-components test-build

# Registry configuration
REGISTRY ?= ghcr.io
NAMESPACE ?= wasmcp
VERSION ?= 0.1.0

# Component directories
TOOL_COMPONENTS = $(shell cd tools 2>/dev/null && ls -d */ 2>/dev/null | sed 's/\///g' || echo "")
COMPOSED_COMPONENTS = $(shell cd composed 2>/dev/null && ls -d */ 2>/dev/null | sed 's/\///g' || echo "")

ALL_COMPONENTS = $(addprefix tools/,$(TOOL_COMPONENTS)) $(addprefix composed/,$(COMPOSED_COMPONENTS))

# Colors for output
GREEN = \033[0;32m
YELLOW = \033[0;33m
BLUE = \033[0;34m
NC = \033[0m # No Color

# Default target
all: build

# Display help information
help:
	@echo "$(BLUE)wasmcp Components Build & Publish System$(NC)"
	@echo ""
	@echo "$(GREEN)Available targets:$(NC)"
	@echo "  make setup              - Install required Rust toolchain"
	@echo "  make build              - Build all components"
	@echo "  make build-component    - Build specific component (COMPONENT=tools/name or composed/name)"
	@echo "  make clean              - Clean all build artifacts"
	@echo "  make publish            - Publish all components to OCI registry"
	@echo "  make publish-component  - Publish specific component (COMPONENT=tools/name or composed/name)"
	@echo "  make list-components    - List all available components"
	@echo "  make test-build         - Test build without publishing"
	@echo ""
	@echo "$(GREEN)Configuration:$(NC)"
	@echo "  REGISTRY  = $(REGISTRY)"
	@echo "  NAMESPACE = $(NAMESPACE)"
	@echo "  VERSION   = $(VERSION)"
	@echo ""
	@echo "$(GREEN)Examples:$(NC)"
	@echo "  make build-component COMPONENT=tools/math"
	@echo "  make publish-component COMPONENT=composed/route-optimizer VERSION=0.2.0"
	@echo "  make publish REGISTRY=ghcr.io NAMESPACE=myorg VERSION=1.0.0"

# List all components
list-components:
	@echo "$(BLUE)Tool Components (use bindings::exports::wasmcp::protocol::tools::Guest):$(NC)"
	@for comp in $(TOOL_COMPONENTS); do echo "  - tools/$$comp"; done
	@echo ""
	@echo "$(BLUE)Composed Components (use bindings::exports::wasmcp::server::handler::Guest):$(NC)"
	@for comp in $(COMPOSED_COMPONENTS); do echo "  - composed/$$comp"; done

# Install required tools
setup:
	@echo "$(YELLOW)Installing Rust wasm32-wasip2 target...$(NC)"
	@rustup target add wasm32-wasip2
	@echo "$(GREEN)Setup complete!$(NC)"

# Build all components using wash build
build: setup
	@echo "$(YELLOW)Building all components with wash build...$(NC)"
	@for comp in $(ALL_COMPONENTS); do \
		echo "$(BLUE)Building $$comp...$(NC)"; \
		cd $$comp && wash build && cd ../..; \
	done
	@echo "$(GREEN)All components built successfully!$(NC)"

# Build a specific component
build-component:
	@if [ -z "$(COMPONENT)" ]; then \
		echo "$(YELLOW)Error: COMPONENT not specified$(NC)"; \
		echo "Usage: make build-component COMPONENT=<tools/name or composed/name>"; \
		exit 1; \
	fi
	@echo "$(YELLOW)Building $(COMPONENT)...$(NC)"
	@cd $(COMPONENT) && wash build
	@echo "$(GREEN)$(COMPONENT) built successfully!$(NC)"

# Clean all build artifacts
clean:
	@echo "$(YELLOW)Cleaning all components...$(NC)"
	@for comp in $(ALL_COMPONENTS); do \
		echo "$(BLUE)Cleaning $$comp...$(NC)"; \
		cd $$comp && cargo clean && cd ../..; \
	done
	@echo "$(GREEN)Clean complete!$(NC)"

# Publish all components to OCI registry
publish: build
	@echo "$(YELLOW)Publishing all components to $(REGISTRY)/$(NAMESPACE)...$(NC)"
	@for comp in $(ALL_COMPONENTS); do \
		comp_name=$$(basename $$comp); \
		echo "$(BLUE)Publishing $$comp_name@$(VERSION)...$(NC)"; \
		./scripts/publish.sh $$comp $(VERSION) $(REGISTRY) $(NAMESPACE); \
	done
	@echo "$(GREEN)All components published successfully!$(NC)"

# Publish a specific component
publish-component:
	@if [ -z "$(COMPONENT)" ]; then \
		echo "$(YELLOW)Error: COMPONENT not specified$(NC)"; \
		echo "Usage: make publish-component COMPONENT=<tools/name or composed/name> [VERSION=x.y.z]"; \
		exit 1; \
	fi
	@echo "$(YELLOW)Publishing $(COMPONENT)@$(VERSION) to $(REGISTRY)/$(NAMESPACE)...$(NC)"
	@./scripts/publish.sh $(COMPONENT) $(VERSION) $(REGISTRY) $(NAMESPACE)
	@echo "$(GREEN)$(COMPONENT)@$(VERSION) published successfully!$(NC)"

# Test build without publishing
test-build: build
	@echo "$(GREEN)Test build completed successfully!$(NC)"
	@echo "$(BLUE)Built components can be found in:$(NC)"
	@for comp in $(ALL_COMPONENTS); do \
		comp_name=$$(basename $$comp); \
		if [ -f $$comp/target/wasm32-wasip2/release/$$comp_name.wasm ]; then \
			echo "  $$comp/target/wasm32-wasip2/release/$$comp_name.wasm"; \
		fi; \
	done
