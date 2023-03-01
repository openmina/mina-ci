FROM rust:1.66 as builder

RUN apt update && apt install -y curl jq

FROM builder

WORKDIR /usr/src/mina-aggregator

COPY . .

RUN cargo install --path .

CMD ["mina-aggregator"]

