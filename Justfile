test:
    #!/usr/bin/env bash
    set -e
    trap 'podman-compose down -v' EXIT
    podman-compose up --build -d
    until podman exec postgres_test pg_isready -U test_user; do sleep 1; done
    until podman exec mysql_test mysqladmin ping --silent; do sleep 1; done
    TEST_DB_PASSWORD=$(podman exec postgres_test printenv POSTGRES_PASSWORD) \
    TEST_MYSQL_PASSWORD=$(podman exec mysql_test printenv MYSQL_PASSWORD) \
    cargo test -v

up:
    podman-compose up -d postgres_test

connect:
    until podman exec postgres_test pg_isready -U test_user; do sleep 1; done
    pgcli -h localhost -p 5439 -U test_user -d postgres_test

down:
    podman-compose down -v

build:
    cargo build --release

dev:
    cargo run --release
