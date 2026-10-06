# syntax=docker/dockerfile:1.7
#
# Stages:
#   builder  - Rust toolchain + maturin; runs cargo tests and builds the wheel
#   runtime  - slim Python with only the wheel installed (what gets deployed)
#   test     - runtime + pytest; `docker build --target test .` is the no-toolchain test path

ARG PYTHON_VERSION=3.13

FROM python:${PYTHON_VERSION}-slim-bookworm AS builder
RUN apt-get update \
 && apt-get install -y --no-install-recommends curl build-essential pkg-config ca-certificates \
 && rm -rf /var/lib/apt/lists/*
RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
ENV PATH=/root/.cargo/bin:$PATH
RUN pip install --no-cache-dir "maturin>=1.8,<2"

WORKDIR /src
COPY Cargo.toml rust-toolchain.toml pyproject.toml README.md ./
COPY Cargo.lock* ./
COPY crates ./crates
COPY python ./python

RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo test --workspace --locked \
 && maturin build --release --locked --out /wheels

FROM python:${PYTHON_VERSION}-slim-bookworm AS runtime
RUN useradd --create-home --uid 10001 app
COPY --from=builder /wheels /wheels
RUN pip install --no-cache-dir /wheels/*.whl && rm -rf /wheels
USER app
WORKDIR /home/app
CMD ["python", "-c", "import genoforge; print(genoforge.core_version())"]

FROM runtime AS test
USER root
RUN pip install --no-cache-dir "pytest>=8" "hypothesis>=6"
USER app
COPY --chown=app pyproject.toml ./
COPY --chown=app tests ./tests
RUN python -m pytest
