test:
    podman-compose up --build -d # поднимаем тестовое бд, к которому будем подключаться
    until podman exec postgres_test pg_isready -U test; do sleep 1; done
    TEST_DB_PASSWORD=$(podman exec postgres_test printenv POSTGRES_PASSWORD) cargo test -v
    podman-compose down -v

up:
    podman-compose up -d postgres_test

connect:
    until podman exec postgres_test pg_isready -U test; do sleep 1; done
    pgcli -h localhost -p 5432 -U test_user -d postgres_test

down:
    podman-compose down -v

build:
    cargo build --release

dev:
    cargo run --release
