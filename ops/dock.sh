#!/usr/bin/env bash
# docker-stack.sh — helper script for full Docker Compose lifecycle

# 1. Build images
# Rebuild all services’ images (or pass SERVICE names after --quiet-pull to limit)
build() {
    echo "🔨 Building images…"
    docker compose build --parallel
}

# 2. Start services
# Create & start containers in detached mode
up() {
    echo "🚀 Bringing up services…"
    docker compose up -d
}

# 3. View logs
# Follow logs for all services (or specify one: logs web)
logs() {
    echo "📜 Tailing logs…"
    docker compose logs -f "$@"
}

# 4. Exec into container
# e.g. exec SERVICE bash
exec_() {
    SERVICE="${1:?service name required}"
    shift
    echo "🖥️  Exec into $SERVICE: ${*:-bash}"
    docker compose exec "$SERVICE" "${@:-bash}"
}

# 5. Stop services
# Stop containers but keep them around
stop() {
    echo "✋ Stopping services…"
    docker compose stop
}

# 6. Tear down
# Stop and remove containers, networks, volumes, and images if desired
down() {
    echo "🧹 Tearing down stack…"
    # remove containers, default network
    docker compose down
}

# 7. Remove all volumes & images
# Use with care: deletes named volumes & built images
clean() {
    echo "🗑️  Removing volumes and images…"
    docker compose down --volumes --rmi all
}

# 8. System-wide prune unused data
# Deletes all stopped containers, unused networks, dangling images and build cache
prune() {
    echo "⚠️  Pruning unused Docker objects…"
    docker system prune -af --volumes
}

# Dispatch based on first argument
case "${1:-}" in
build) build ;;
up) up ;;
logs)
    shift
    logs "$@"
    ;;
exec)
    shift
    exec_ "$@"
    ;;
stop) stop ;;
down) down ;;
clean) clean ;;
prune) prune ;;
*)
    cat <<EOF
Usage: $(basename "$0") <command> [args]

Commands:
  build           Build all service images
  up              Start services (detached)
  logs [SERVICE]  Follow logs (all or specific SERVICE)
  exec SERVICE    Exec into SERVICE container
  stop            Stop services
  down            Stop & remove containers & networks
  clean           down + remove volumes & images
  prune           System prune unused containers, images, networks, volumes
EOF
    exit 1
    ;;
esac
