.DEFAULT_GOAL := help
SHELL := /bin/bash
.SHELLFLAGS := -eu -o pipefail -c

VENV ?= .venv
PY   := $(VENV)/bin/python

.PHONY: help setup develop test test-rust test-py lint fmt up down clean docker-test

help: ## list targets
	@grep -E '^[a-zA-Z_-]+:.*?## ' $(MAKEFILE_LIST) | awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}'

setup: ## create the venv and install the package (builds the Rust extension) + dev tools
	uv venv $(VENV) --python 3.12 --allow-existing
	uv pip install --python $(PY) -e '.[dev]'

develop: ## rebuild the Rust extension in place after editing crates/
	uv run --python $(PY) maturin develop --release

test: test-rust test-py ## run everything

test-rust: ## cargo tests for all crates
	cargo test --workspace

test-py: ## pytest against the installed extension
	$(PY) -m pytest

lint: ## clippy (pedantic, deny warnings), rustfmt check, ruff, mypy
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	$(PY) -m ruff check .
	$(PY) -m ruff format --check .
	$(PY) -m mypy

fmt: ## apply rustfmt + ruff formatting
	cargo fmt --all
	$(PY) -m ruff format .
	$(PY) -m ruff check --fix .

up: ## start postgres + mongo
	docker compose up -d --wait

down: ## stop services (keeps volumes)
	docker compose down

docker-test: ## build the image and run the Rust + Python test suites inside it (no local toolchain needed)
	docker build --target test -t genoforge:test .

clean: ## remove build artefacts
	rm -rf target $(VENV) .pytest_cache .mypy_cache .ruff_cache .hypothesis
