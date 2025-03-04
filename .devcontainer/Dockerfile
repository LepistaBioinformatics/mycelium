FROM rust:latest

WORKDIR /workspace

RUN apt-get update && apt-get install -y curl

COPY . .

RUN cargo fetch

CMD ["sleep", "infinity"]
