#!/bin/sh
docker run -d --name sqlx-postgres -e POSTGRES_PASSWORD=postgres -p 5432:5432 \
  --health-cmd pg_isready --health-interval 10s --health-timeout 5s --health-retries 5 postgres
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres
until docker inspect --format='{{json .State.Health.Status}}' sqlx-postgres | grep "healthy"
do
echo "Waiting for database to become healthy..."
sleep 0.3
done
cargo sqlx mig run
cargo sqlx prepare --workspace -- --all-features --all-targets
docker stop sqlx-postgres
docker rm sqlx-postgres
