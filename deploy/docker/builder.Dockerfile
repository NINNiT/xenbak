FROM rust:1.82.0 as rust_builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin xenbakd

FROM alpine:latest as xe_bin_builder
WORKDIR /app 
RUN apk add --no-cache curl p7zip
COPY deploy/xe/extract_xe_bin.sh /app/xe.sh
RUN chmod +x /app/xe.sh && cd /app && sh /app/xe.sh 

FROM debian:bookworm as runtime
COPY ./target/release/xenbakd /usr/bin/xenbakd
RUN apt-get update && apt-get install -y curl borgbackup stunnel ssh && apt-get clean
RUN mkdir -p /etc/xenbakd
COPY config.toml /etc/xenbakd/config.toml
COPY --from=xe_bin_builder /app/xe /usr/bin/xe

CMD ["/usr/bin/xenbakd", "--config",  "/etc/xenbakd/config.toml", "daemon"]

