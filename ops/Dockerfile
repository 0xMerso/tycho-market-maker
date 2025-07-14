# Single-stage Dockerfile using rust:slim
FROM rust:slim

# Install system dependencies for build and runtime
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      libssl-dev \
      pkg-config \
      build-essential \
      curl \
      git && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

# Set workspace
WORKDIR /workspace

# Copy entire project
COPY . .

# Remove any local Cargo override
RUN rm -f .cargo/config.toml

# Accept build args
ARG BUILD_TYPE=release
ARG PROGRAM=monitor
ENV BUILD_TYPE=${BUILD_TYPE}
ENV PROGRAM=${PROGRAM}

RUN echo "🐳  BUILD_TYPE: $BUILD_TYPE"
RUN echo "🐳  PROGRAM: $PROGRAM"

# Build the Rust binary
RUN if [ "$BUILD_TYPE" = "release" ]; then \
      cargo build --release --bin ${PROGRAM}; \
    else \
      cargo build --bin ${PROGRAM}; \
    fi

# Default command: run built binary based on BUILD_TYPE
CMD [ "bash", "-c", "if [ \"$BUILD_TYPE\" = \"release\" ]; then exec ./target/release/$PROGRAM; else exec ./target/debug/$PROGRAM; fi" ]