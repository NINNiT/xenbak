FROM alpine:latest as xe_bin_builder
WORKDIR /app 
RUN apk add --no-cache curl p7zip
COPY deploy/xe/extract_xe_bin.sh /app/xe.sh
RUN chmod +x /app/xe.sh && cd /app && sh /app/xe.sh 


FROM debian:bookworm as runtime
LABEL org.opencontainers.image.source=https://github.com/ninnit/xenbak
LABEL org.opencontainers.image.description="Backup tool for xen/xcp-ng hypervisors"

RUN apt-get update && apt-get install -y curl borgbackup stunnel ssh && apt-get clean

COPY ./target/x86_64-unknown-linux-musl/release/xenbakd /usr/bin/xenbakd
RUN chmod +x /usr/bin/xenbakd
RUN mkdir -p /etc/xenbakd
COPY config.toml /etc/xenbakd/config.toml
COPY --from=xe_bin_builder /app/xe /usr/bin/xe

ENTRYPOINT ["/usr/bin/xenbakd"]
CMD ["--config", "/etc/xenbakd/config.toml", "daemon"]
