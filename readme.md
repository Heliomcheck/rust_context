Сервер на rust для управления событиями, чатом, пользователями
---
Client: https://github.com/CQuann/Krug.git

## Технологии
- Rust + Tokio (асинхронный рантайм)
- Axum (веб‑фреймворк)
- SQLx (асинхронный драйвер PostgreSQL)
- PostgreSQL (основная БД)
- Docker / Docker Compose
- WebSocket (чат в реальном времени)

## Install:
```bash
# For Arch Linux
docker-compose up -d  # for database
mv env .env # for set .env file
sudo pacman -S sqlx-cli
sudo chmod +x migrate_db.sh
./migrate_db.sh
cargo install --path .
```
```bash
# For windows (Ubuntu 26.04 WSL)
sudo apt update && sudo apt upgrade -y
sudo apt install -y ca-certificates gnupg curl
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
sudo apt update
sudo apt install -y docker-ce-cli docker-compose-plugin rustc cargo pkg-config libssl-dev build-essential
sudo systemctl restart docker
sudo docker compose up -d # for database
mv env .env # for set .env file
cargo install sqlx-cli --version 0.8.2
export PATH="$HOME/.cargo/bin:$PATH"
sudo chmod +x migrate_db.sh
./migrate_db.sh
cp -r /"your path to project"/rust_context ~/rust_context
cd ~/rust_context
cargo install --path .
```

## Usage:
```bash
rust_context ip:port
```

## Tests:
```bash
cargo test -- --test-threads=1
```