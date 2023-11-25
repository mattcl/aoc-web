#!/bin/sh
set -ex

if [ "$LINT" -eq 1 ]; then
    # make sure we're formatted
    cargo fmt --check

    # fail on clippy warnings
    cargo clippy -- -Dwarnings
fi

# ensure we can build
cargo build --verbose ${EXTRA_CARGO_BUILD_FLAGS}

# we're going to install postgres because having supporting services in
# concourse kind of sucks
if [ -f /sbin/apk ]; then
    apk update
    apk add postgresql

    mkdir /run/postgresql
    chown postgres:postgres /run/postgresql
else
    apt-get update
    apt-get install -y postgresql
fi

# permit all
set -- /etc/postgresql/*/main/pg_hba.conf
cat > "$1" <<-EOF
host   all   postgres   localhost   trust

EOF
service postgresql restart

psql -h localhost -U postgres -c 'create database aoc;'

export DATABASE_URL="postgres://postgres@localhost/aoc"

# ensure tests pass
cargo test --verbose ${EXTRA_CARGO_TEST_FLAGS}
