test:
    podman-compose up --build -d # поднимаем тестовое бд, к которому будем подключаться
    cargo test -v
    podman-compose down -v

buid:
    cargo build --release
