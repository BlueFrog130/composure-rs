# docker build -t {{project-name}} .
# docker run -p 8080:8080 -e DISCORD_PUBLIC_KEY=... -e DISCORD_TOKEN=... {{project-name}}
#
# Plain HTTP on $PORT: put it behind a TLS-terminating proxy or load balancer. `GET /health`
# answers 200 for health checks; SIGTERM drains in-flight requests for up to 30 seconds.

FROM rust:1-slim AS build
WORKDIR /app
COPY . .
RUN cargo build --release --bin {{project-name}}

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/{{project-name}} /usr/local/bin/bot
ENV PORT=8080
EXPOSE 8080
USER nobody
CMD ["bot"]
