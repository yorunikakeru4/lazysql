test:
    podman-compose up --build -d # поднимаем тестовое бд, к которому будем подключаться
    until podman exec postgres_test pg_isready -U test; do sleep 1; done
    cargo test -v
    podman-compose down -v

run_postgres:
    podman-compose up -d postgres_test
    until podman exec postgres_test pg_isready -U test; do sleep 1; done
    echo "Test PostgreSQL is ready. Password: vBnA46MVSs"
    pgcli -h localhost -p 5432 -U test_user -d db_test

build:
    cargo build --release
