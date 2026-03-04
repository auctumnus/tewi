dev:
  just db
  @echo "Starting the app..."
  concurrently --names 後,前 "just dev-backend" "just dev-frontend"

dev-backend:
  watchexec -w templates -w src -r cargo run -- --attachments-folder uploads/attachments --thumbnails-folder uploads/thumbnails

dev-frontend:
  cd frontend && bun run dev

# Build the application
build:
    cargo build

# Build frontend assets
build-frontend:
    cd frontend && bun run build

# Build everything (backend + frontend)
build-all:
    just build-frontend
    just build

# Build the Docker image
docker-build:
    docker build -t tewi .

# View logs for the full stack
logs:
    docker compose logs -f

# Clean up all containers and volumes
clean:
    docker compose down -v
    docker system prune -f

# === Database ===

# Start only the database
db:
    @echo "Starting PostgreSQL database..."
    docker compose up postgres -d
    @echo "Waiting for database to be ready..."
    @until docker exec tewi-db pg_isready -U user -d tewi >/dev/null 2>&1; do \
        echo "Database is unavailable - sleeping"; \
        sleep 1; \
    done
    @echo "Running migrations..."
    just db-migrate
    @echo "Database is ready!"

db-migrate:
  sqlx migrate run

# View database logs only
db-logs:
    docker compose logs postgres -f

# Stop the database
db-stop:
    docker compose stop postgres

# Reset database (stop, remove volume, start fresh)
db-reset:
    docker compose down postgres -v
    just db

# Reset database, add example boards, create admin account
init-dev:
    just db-reset
    just build
    target/debug/tewi admin new --name admin --password password
    target/debug/tewi category new --name Interests
    target/debug/tewi board new --name Technology --slug g --description Technology --category Interests
    target/debug/tewi board new --name Outdoors --slug out --description Outdoors --category Interests
    target/debug/tewi board new --name Miscellaneous --slug misc --description "Miscellaneous posts"