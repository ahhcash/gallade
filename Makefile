# phony targets (not actual files)
.PHONY: all build test lint clean check release debug install help watch bench profile docs

# config
CARGO := cargo
RUSTC := rustc
RUSTFMT := rustfmt
CLIPPY := clippy
TARGET_DIR := target

# default target when you just run 'make'
all: check build test

# build that bad boy
build:
	$(CARGO) build

# run tests with pretty output
test:
	$(CARGO) test -- --nocapture

# check if it compiles without actually building
check:
	$(CARGO) check

# run clippy for linting
lint:
	$(CARGO) clippy -- -D warnings

# format the code
fmt:
	$(CARGO) fmt

# clean build artifacts
clean:
	$(CARGO) clean
	rm -rf $(TARGET_DIR)

# build for release with optimizations
release:
	$(CARGO) build --release

# build with debug symbols
debug:
	$(CARGO) build --debug

# install binary globally
install:
	$(CARGO) install --path .

# run with file watching for development
watch:
	$(CARGO) watch -x run

# run benchmarks
bench:
	$(CARGO) bench

# generate cpu profile
profile:
	$(CARGO) build --release
	perf record -g target/release/gallade
	perf report -g 'graph,0.5,caller'

# generate docs
docs:
	$(CARGO) doc --no-deps --open

# runs before pushing to main
pre-push: fmt lint test build
	@echo "✨ all good to push!"

# help target shows available commands
help:
	@echo "available targets:"
	@echo "  make          : runs check, build and test"
	@echo "  make build    : builds the project"
	@echo "  make test     : runs tests"
	@echo "  make lint     : runs clippy"
	@echo "  make fmt      : formats code"
	@echo "  make clean    : removes build artifacts"
	@echo "  make release  : builds with optimizations"
	@echo "  make watch    : runs with file watching"
	@echo "  make bench    : runs benchmarks"
	@echo "  make profile  : generates cpu profile"
	@echo "  make docs     : generates documentation"
	@echo "  make pre-push : runs formatting, lint and tests"