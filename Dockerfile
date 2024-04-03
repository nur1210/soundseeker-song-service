FROM rust:latest

ENV SQLX_OFFLINE=true

WORKDIR /usr/src/song_service

COPY . .

RUN apt-get update && apt-get install protobuf-compiler -y

RUN cargo install --path .

CMD ["song_service"]

EXPOSE 50051
