set export

DATABASE_URL := "postgres://aoc:sandcastle@localhost/aoc"

# watch and run tests
watch:
    DATABASE_URL="postgres://postgres:postgres@localhost/aoc" cargo watch -x test

# run tests with coverage reporting
coverage:
    DATABASE_URL="postgres://postgres:postgres@localhost/aoc" cargo tarpaulin

# build the docker images
docker-build:
    docker build .

# bring up everything in the stack but the app
dev +CMD:
    docker compose --profile=dev {{ CMD }}

# bring up the entire local stack (with containerized app)
full +CMD:
    docker compose --profile=full {{ CMD }}

# run a sqlx migrate command
migrate +CMD:
    sqlx migrate {{ CMD }}

# run a sqlx database command
db +CMD:
    sqlx database {{ CMD }}
