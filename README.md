# gallade 🗡️

A modern, high-performance dependency manager for Java. Inspired by bun's speed and simplicity, gallade aims to make Java dependency management fast and painless.

## Why Gallade?

- Fast dependency resolution with direct jar downloads
- Clean lockfile for reproducible builds
- Modern CLI interface
- Zero configuration needed
- Works alongside existing Maven projects

## Status: Work in Progress

Currently implemented:
- Basic dependency resolution
- Lockfile generation
- Local artifact caching
- Maven Central support
- Java project initialization

Coming soon:
- Dependency tree visualization
- Parallel downloads
- Conflict resolution
- Proxy support
- Checksum verification
- Gradle support

## Quick Start

```bash
# Initialize a new project
gallade init --groupId com.example --artifactId my-project

# Add a dependency
gallade add com.google.guava:guava

# Build your project
gallade build

# Run your project
gallade run
```

## How It Works

Gallade takes inspiration from modern package managers like bun and uv, focusing on speed and simplicity:

1. Downloads JARs directly from Maven Central
2. Maintains an efficient local cache
3. Uses a lockfile for reproducible builds
4. Handles dependency resolution locally for better performance

## Contributing

Please open any PR and I'll merge it

## Acknowledgments

Inspired by uv (Python package manager) and the desire for faster Java dependency resolution.

## License

MIT

