# wasmcp Tools Repository

**Reusable WebAssembly components for building composable MCP tool servers**

This repository contains production-ready wasmcp components organized by type and ready to publish to OCI registries.

## Quick Start

```bash
# Build all components
make build

# Publish a component
make publish-component COMPONENT=tools/math VERSION=0.1.0

# Use published components
wasmcp compose wasmcp:math@0.1.0 wasmcp:pythagorean-middleware@0.1.0 -o server.wasm
```

## Repository Structure

```
tools/
├── tools/              # Primitive tool components (protocol::tools::Guest)
│   ├── math/
│   ├── statistics/
│   ├── string-utils/
│   ├── system-info/
│   └── geospatial-*/
└── composed/           # Middleware components (server::handler::Guest)
    ├── pythagorean-middleware/
    ├── distance-calculator/
    ├── variance-middleware/
    ├── stddev-middleware/
    └── route-optimizer/
```

## Component Types

### Tools (Primitive Components)

Located in `tools/` directory. These export `wasmcp:protocol/tools` and provide atomic operations.

**Guest trait:** `use bindings::exports::wasmcp::protocol::tools::Guest;`

**Examples:**
- `tools/math` - Mathematical operations (add, multiply, square, etc.)
- `tools/statistics` - Statistical primitives (mean, sum, count)
- `tools/string-utils` - String manipulation
- `tools/geospatial-distance` - Distance calculations

### Composed (Middleware Components)

Located in `composed/` directory. These import/export `wasmcp:server/handler` to orchestrate downstream tools.

**Guest trait:** `use bindings::exports::wasmcp::server::handler::Guest;`

**Examples:**
- `composed/pythagorean-middleware` - Composes square + square_root
- `composed/route-optimizer` - Composes geospatial tools
- `composed/stddev-middleware` - Composes variance + square_root

## Building Components

### Build All

```bash
make build
```

### Build Specific Component

```bash
make build-component COMPONENT=tools/math
make build-component COMPONENT=composed/route-optimizer
```

### List Components

```bash
make list-components
```

## Publishing

See [PUBLISHING.md](PUBLISHING.md) for complete publishing guide.

### Quick Publish

```bash
# Publish to GitHub Container Registry
make publish-component COMPONENT=tools/math VERSION=0.1.0

# Publish all components
make publish VERSION=0.1.0

# Custom registry
make publish-component \
  COMPONENT=tools/math \
  VERSION=0.1.0 \
  REGISTRY=ghcr.io \
  NAMESPACE=myorg
```

## Using Published Components

```bash
# Use in compositions
wasmcp compose \
  wasmcp:pythagorean-middleware@0.1.0 \
  wasmcp:math@0.1.0 \
  -o server.wasm

# Mix local and published
wasmcp compose \
  ./my-local-middleware \
  wasmcp:math@0.1.0 \
  -o server.wasm
```

## Pipeline Ordering

**IMPORTANT:** Middleware must come BEFORE the tools it needs.

```bash
# ✅ Correct
wasmcp compose wasmcp:pythagorean-middleware@0.1.0 wasmcp:math@0.1.0

# ❌ Wrong
wasmcp compose wasmcp:math@0.1.0 wasmcp:pythagorean-middleware@0.1.0
```

## CI/CD

GitHub Actions automatically builds and publishes components on:
- **Tags** (`v*.*.*`) - Publishes all components
- **Pull Requests** - Test builds only
- **Manual dispatch** - Selective publishing

Create a release:

```bash
./scripts/update-versions.sh 0.2.0
git add .
git commit -m "Bump version to 0.2.0"
git tag v0.2.0
git push && git push --tags
```

## Available Components

### Tool Components

| Component | Description |
|-----------|-------------|
| `tools/math` | Mathematical operations |
| `tools/statistics` | Statistical primitives |
| `tools/string-utils` | String manipulation |
| `tools/system-info` | System utilities |
| `tools/geospatial-distance` | Distance calculations |
| `tools/geospatial-bearing` | Bearing calculations |
| `tools/geospatial-point-in-polygon` | Geospatial queries |

### Composed Components

| Component | Dependencies |
|-----------|-------------|
| `composed/pythagorean-middleware` | `tools/math` |
| `composed/distance-calculator` | `tools/math` |
| `composed/variance-middleware` | `tools/statistics` |
| `composed/stddev-middleware` | `composed/variance-middleware`, `tools/math` |
| `composed/route-optimizer` | `tools/geospatial-*`, `tools/math` |

## Development

### Adding a New Tool Component

1. Create in `tools/` directory
2. Use `bindings::exports::wasmcp::protocol::tools::Guest`
3. Update `Makefile` if needed (auto-detected)

### Adding a New Composed Component

1. Create in `composed/` directory
2. Use `bindings::exports::wasmcp::server::handler::Guest`
3. Import `wasmcp:server/handler` for downstream calls
4. Update `Makefile` if needed (auto-detected)

## License

See [LICENSE](LICENSE) for details.
