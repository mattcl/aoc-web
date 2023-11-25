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
    mkdir /var/lib/postgresql/data/
    chown postgres:postgres /var/lib/postgresql/data/
    touch var/lib/postgresql/.psql_history
    chown postgres:postgres /var/lib/postgresql/data/.psql_history
    chmod 0700 /var/lib/postgresql/data/

    su postgres -c 'initdb -D /var/lib/postgresql/data'
    # permit all
    echo "host all postgres localhost trust" >> /var/lib/postgresql/data/pg_hba.conf
    su postgres -c 'pg_ctl start -D /var/lib/postgresql/data'
else
    apt-get update
    apt-get install -y postgresql

    # permit all
    hba=$(find /etc/postgresql -name 'pg_hba.conf')
    cat > "$hba" <<-EOF
host   all   postgres   localhost   trust
EOF
fi

service postgresql restart

psql -h localhost -U postgres -c 'create database aoc;'

export DATABASE_URL="postgres://postgres@localhost/aoc"

# ensure tests pass
cargo test --verbose ${EXTRA_CARGO_TEST_FLAGS}
