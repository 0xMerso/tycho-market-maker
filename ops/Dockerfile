# Builder stage: use the official Rust+Alpine image for a minimal toolchain
FROM rust:1.85-alpine AS builder

# install native build-deps plus Node.js/npm for Prisma
RUN apk add --no-cache \
  build-base \
  openssl-dev \
  pkgconfig \
  curl \
  git \
  nodejs \
  npm

WORKDIR /workspace

# copy only manifests to cache Rust & JS dependencies
COPY Cargo.toml Cargo.lock ./
COPY package*.json ./
COPY prisma ./prisma

# install JS deps and generate Prisma client
# RUN npm ci --only=production && npx prisma generate

# now copy full source
COPY . .

# remove any local override
RUN rm -f .cargo/config.toml

# accept build args
ARG BUILD_TYPE=debug
ARG PROGRAM

# build the program
RUN cargo build --bin "$PROGRAM"

# Runtime stage: tiny Alpine with only needed libraries
FROM alpine:3.18

# install runtime SSL and git
RUN apk add --no-cache openssl git

WORKDIR /workspace

# re-declare args so we can use them here
ARG BUILD_TYPE=debug
ARG PROGRAM

# copy the compiled binary from builder
COPY --from=builder /workspace/target/${BUILD_TYPE}/$PROGRAM /usr/local/bin/$PROGRAM

# entrypoint just runs your program
ENTRYPOINT ["/usr/local/bin/"]
CMD ["${PROGRAM}"]
