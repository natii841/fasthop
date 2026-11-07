# ---- dashboard ----
FROM node:24-alpine AS dashboard
WORKDIR /app/dashboard
COPY dashboard/package.json dashboard/tsconfig.json dashboard/vite.config.ts dashboard/index.html ./
COPY dashboard/src ./src
RUN npm install && npm run build

# ---- Rust build ----
FROM rust:1.88-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY crates ./crates
COPY --from=dashboard /app/dashboard/dist ./dashboard/dist
RUN cargo build --release --locked -p fasthop-core

# ---- runtime ----
FROM alpine:3.21
RUN apk add --no-cache ca-certificates
WORKDIR /app
COPY --from=build /app/target/release/fasthop-core ./fasthop-core
COPY --from=dashboard /app/dashboard/dist ./dashboard/dist
COPY routing.example.json ./routing.example.json
ENV RUST_LOG=info
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s CMD wget -qO- http://127.0.0.1:8080/healthz || exit 1
CMD ["./fasthop-core"]
