FROM rust:1.66

WORKDIR /usr/src/mina-aggregator

COPY . .

RUN cargo install --path .

CMD ["mina-aggregator"]

