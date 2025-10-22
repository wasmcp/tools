#!/bin/bash

# publish.sh - Publish a wasmcp component to an OCI registry
#
# Usage: ./publish.sh <component-path> <version> [registry] [namespace]
#
# Example:
#   ./publish.sh tools/math 0.1.0
#   This publishes as: ghcr.io/wasmcp/math:0.1.0

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Fixed repository values
GITHUB_USER="wasmcp"
REPO_NAME="tools"
REPO_URL="https://github.com/${GITHUB_USER}/${REPO_NAME}"

# Parse arguments
COMPONENT_PATH="${1:-}"
VERSION="${2:-0.1.0}"
REGISTRY="${3:-ghcr.io}"
NAMESPACE="${4:-wasmcp}"

# Extract component name from path (e.g., tools/math -> math)
COMPONENT_NAME=$(basename "${COMPONENT_PATH}")

# Validate component path
if [ -z "$COMPONENT_PATH" ]; then
    echo -e "${RED}Error: Component path required${NC}"
    echo "Usage: $0 <component-path> [version] [registry] [namespace]"
    echo "Example: $0 tools/math 0.1.0"
    exit 1
fi

# Check if component directory exists
if [ ! -d "$COMPONENT_PATH" ]; then
    echo -e "${RED}Error: Component directory '$COMPONENT_PATH' not found${NC}"
    exit 1
fi

# Validate version
if [ -z "$VERSION" ]; then
    echo -e "${RED}Error: VERSION is required${NC}"
    echo "Usage: $0 <component-path> <version>"
    exit 1
fi

# Construct OCI reference
OCI_REF="${REGISTRY}/${NAMESPACE}/${COMPONENT_NAME}:${VERSION}"

echo -e "${BLUE}Publishing Component${NC}"
echo -e "  Component:  ${GREEN}${COMPONENT_NAME}${NC}"
echo -e "  Path:       ${GREEN}${COMPONENT_PATH}${NC}"
echo -e "  Version:    ${GREEN}${VERSION}${NC}"
echo -e "  OCI Ref:    ${GREEN}${OCI_REF}${NC}"
echo -e "  Repository: ${GREEN}${REPO_URL}${NC}"
echo ""

# Navigate to component directory
cd "$COMPONENT_PATH"

# Ensure build directory exists
mkdir -p build

# wash build converts hyphens to underscores and adds _s suffix for signed components
# e.g., geospatial-bearing -> geospatial_bearing_s.wasm
COMPONENT_WASM_NAME="${COMPONENT_NAME//-/_}_s.wasm"
WASM_PATH="build/${COMPONENT_WASM_NAME}"

# Check if wasm artifact exists (wash build outputs to build/ directory)
if [ ! -f "$WASM_PATH" ]; then
    echo -e "${YELLOW}Component not built, building now...${NC}"
    if ! wash build; then
        echo -e "${RED}Error: wash build failed${NC}"
        exit 1
    fi
fi

# Verify wasm file exists after build
if [ ! -f "$WASM_PATH" ]; then
    echo -e "${RED}Error: WASM artifact not found at ${WASM_PATH}${NC}"
    echo -e "${RED}wash build may have failed or output to unexpected location${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Using WASM artifact: ${WASM_PATH}${NC}"

# Update wasmcloud.toml version if it exists
if [ -f "wasmcloud.toml" ]; then
    echo -e "${YELLOW}Updating version in wasmcloud.toml...${NC}"
    # Use sed to update version, compatible with both GNU and BSD sed
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS (BSD sed)
        sed -i '' "s/^version = .*/version = \"${VERSION}\"/" wasmcloud.toml
    else
        # Linux (GNU sed)
        sed -i "s/^version = .*/version = \"${VERSION}\"/" wasmcloud.toml
    fi
fi

# Check if wash is installed
if ! command -v wash &> /dev/null; then
    echo -e "${RED}Error: 'wash' not found in PATH${NC}"
    echo "Install from: https://wasmcloud.com/docs/installation"
    exit 1
fi

# Check if wkg is installed
if ! command -v wkg &> /dev/null; then
    echo -e "${RED}Error: 'wkg' not found in PATH${NC}"
    echo "Install with: cargo install wkg"
    exit 1
fi

# Publish to OCI registry using wkg oci push with GitHub annotation
echo -e "${YELLOW}Publishing to OCI registry...${NC}"

# Use absolute path for WASM file
WASM_ABSOLUTE_PATH="$(pwd)/${WASM_PATH}"

# Publish using wkg oci push with GitHub repository annotation
if wkg oci push "${OCI_REF}" "${WASM_ABSOLUTE_PATH}" \
    --annotation org.opencontainers.image.source="${REPO_URL}" \
    --annotation org.opencontainers.image.version="${VERSION}"; then

    echo -e "${GREEN}✓ Successfully published ${COMPONENT_NAME}@${VERSION}${NC}"
    echo ""
    echo -e "${BLUE}Component can now be referenced as:${NC}"
    echo -e "  ${GREEN}${NAMESPACE}:${COMPONENT_NAME}@${VERSION}${NC}"
    echo ""
    echo -e "${BLUE}Published to:${NC}"
    echo -e "  ${GREEN}${OCI_REF}${NC}"
    echo ""
    echo -e "${BLUE}Linked to repository:${NC}"
    echo -e "  ${GREEN}${REPO_URL}${NC}"
    echo ""
    echo -e "${BLUE}To use in your project:${NC}"
    echo -e "  ${YELLOW}wasmcp compose ${NAMESPACE}:${COMPONENT_NAME}@${VERSION} ...${NC}"

    cd ..
    exit 0
else
    echo -e "${RED}✗ Failed to publish ${COMPONENT_NAME}${NC}"
    cd ..
    exit 1
fi
