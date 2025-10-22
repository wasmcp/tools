#!/bin/bash

# update-versions.sh - Update version numbers across all components
#
# Usage: ./update-versions.sh <new-version> [component]
#
# Examples:
#   ./update-versions.sh 0.2.0              # Update all components
#   ./update-versions.sh 0.2.0 math         # Update only math component

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
NEW_VERSION="${1:-}"
SPECIFIC_COMPONENT="${2:-}"

# Validate version
if [ -z "$NEW_VERSION" ]; then
    echo -e "${RED}Error: Version required${NC}"
    echo "Usage: $0 <version> [component]"
    echo "Example: $0 0.2.0"
    exit 1
fi

# Validate version format (basic semver check)
if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    echo -e "${YELLOW}Warning: Version '$NEW_VERSION' doesn't match semver format (X.Y.Z)${NC}"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Function to update version in a component
update_component_version() {
    local component=$1
    local version=$2

    if [ ! -d "$component" ]; then
        echo -e "${RED}Error: Component directory '$component' not found${NC}"
        return 1
    fi

    echo -e "${BLUE}Updating $component to version $version...${NC}"

    # Update wasmcloud.toml
    if [ -f "$component/wasmcloud.toml" ]; then
        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' "s/^version = .*/version = \"$version\"/" "$component/wasmcloud.toml"
        else
            sed -i "s/^version = .*/version = \"$version\"/" "$component/wasmcloud.toml"
        fi
        echo -e "  ${GREEN}✓${NC} Updated wasmcloud.toml"
    fi

    # Update Cargo.toml
    if [ -f "$component/Cargo.toml" ]; then
        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' "s/^version = .*/version = \"$version\"/" "$component/Cargo.toml"
        else
            sed -i "s/^version = .*/version = \"$version\"/" "$component/Cargo.toml"
        fi
        echo -e "  ${GREEN}✓${NC} Updated Cargo.toml"
    fi
}

# Get list of components
if [ -n "$SPECIFIC_COMPONENT" ]; then
    COMPONENTS=("$SPECIFIC_COMPONENT")
else
    # Get all component directories (exclude scripts and .github)
    COMPONENTS=()
    for dir in */; do
        dirname="${dir%/}"
        if [ "$dirname" != "scripts" ] && [ "$dirname" != ".github" ]; then
            COMPONENTS+=("$dirname")
        fi
    done
fi

echo -e "${YELLOW}Updating version to: $NEW_VERSION${NC}"
echo ""

# Update all components
for component in "${COMPONENTS[@]}"; do
    update_component_version "$component" "$NEW_VERSION"
done

echo ""
echo -e "${GREEN}Version update complete!${NC}"
echo ""
echo -e "${BLUE}Next steps:${NC}"
echo "  1. Review changes: git diff"
echo "  2. Commit changes: git add . && git commit -m 'Bump version to $NEW_VERSION'"
echo "  3. Create tag: git tag v$NEW_VERSION"
echo "  4. Push changes: git push && git push --tags"
