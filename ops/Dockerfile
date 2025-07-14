# ---------- Builder stage ----------
# Use official Rust + Alpine image for minimal footprint
FROM rust:1.85-alpine AS builder

# Install native build dependencies
RUN apk add --no-cache \
    build-base \
    libusb-dev \
    openssl-dev \
    pkgconfig \
    linux-headers \
    curl \
    git \
    openssl

WORKDIR /workspace

# Copy Rust manifests for dependency caching
COPY Cargo.toml Cargo.lock ./

# Now copy the full project source
COPY . .

# Remove any local Cargo override (if present)
RUN rm -f .cargo/config.toml

# Build arguments: allow debug or release builds
ARG BUILD_TYPE=debug  # use 'release' for optimized binary
ARG PROGRAM           # name of the binary to compile

# Compile the Rust program
RUN if [ "$BUILD_TYPE" = "release" ]; then \
      cargo build --release --bin "$PROGRAM" -q; \
    else \
      cargo build --bin "$PROGRAM" -q; \
    fi


# ---------- Runtime stage ----------
# Start from minimal Alpine image with only runtime libs
FROM alpine:3.18

# Install SSL and git (if needed at runtime by your app)
RUN apk add --no-cache \
    openssl \
    git

WORKDIR /workspace

# Declare build args again to know which binary to copy
ARG BUILD_TYPE=debug
ARG PROGRAM

# Log
RUN echo "🐳 BUILD_TYPE: $BUILD_TYPE"
RUN echo "🐳 PROGRAM: $PROGRAM"

# Copy compiled binary from builder stage
COPY --from=builder /workspace/target/${BUILD_TYPE}/$PROGRAM /usr/local/bin/$PROGRAM

# Set entrypoint to your program
ENTRYPOINT ["/usr/local/bin/$PROGRAM"]
# Optionally allow additional args
CMD ["$PROGRAM"]