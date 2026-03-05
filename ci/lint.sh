#!/bin/sh
set -eu
echo "Checking Rust formatting..."
cargo +nightly fmt --check --all
echo "Checking web formatting..."
npm run prettier-check
echo "Checking build..."
cargo +nightly clippy --all -- -D warnings
echo "Creating test database..."
docker stop sqlx-postgres
docker rm sqlx-postgres
docker run -d --name sqlx-postgres -e POSTGRES_PASSWORD=postgres -p 5432:5432 \
  --health-cmd pg_isready --health-interval 10s --health-timeout 5s --health-retries 5 postgres
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres
until docker inspect --format='{{json .State.Health.Status}}' sqlx-postgres | grep "healthy"
do
echo "Waiting for database to become healthy..."
sleep 0.3
done
echo "Running tests..."
cargo +nightly test --all
echo "Cleaning up..."
docker stop sqlx-postgres
docker rm sqlx-postgres
echo "Done!"