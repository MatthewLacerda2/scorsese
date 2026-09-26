# syntax=docker/dockerfile:1
#
# The React build, served by nginx. Built from the repo root by compose.yaml so
# it can reach both web/ and deploy/nginx.conf; web.Dockerfile.dockerignore
# says what it may see.

FROM oven/bun:1.3.14 AS build
WORKDIR /web
# Dependencies on their own layer: an edit to a component reinstalls nothing.
COPY web/package.json web/bun.lock ./
RUN bun install --frozen-lockfile
COPY web/ ./
RUN bun run build

FROM nginx:1.30-alpine
COPY deploy/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /web/dist /usr/share/nginx/html
