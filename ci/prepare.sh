#!/bin/sh
docker run -d --name sqlx-postgres -e POSTGRES_PASSWORD=postgres -p 5432:5432 postgres
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres
cargo sqlx mig run
cargo sqlx prepare --workspace -- --all-features --all-targets
docker stop sqlx-postgres
docker rm sqlx-postgres
