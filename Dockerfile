FROM rust:1.74-alpine as release

RUN apk add musl-dev

WORKDIR /usr/src/aoc-web
COPY . .
RUN cargo install --locked --target-dir /target --path .

# the prod image
FROM alpine:3.18

ENV AOC_BIND_ADDR=0.0.0.0

RUN adduser -D aoc

COPY --from=release /usr/local/cargo/bin/aoc-web /usr/local/bin/aoc-web

USER aoc

ENTRYPOINT ["aoc-web"]

CMD ["server"]
