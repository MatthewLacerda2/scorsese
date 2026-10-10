# syntax=docker/dockerfile:1
#
# The React build, served by nginx. Built from the repo root by compose.yaml so
# it can reach web/, deploy/nginx.conf and the two things the landing page
# loads from outside web/ (#904): the faces scorsese ships with a video and its
# motion kit, used as they are rather than copied. They sit at the same paths
# relative to web/ as in the repo, so the build resolves them the same way.
# web.Dockerfile.dockerignore says what it may see.

FROM oven/bun:1.3.14 AS build
WORKDIR /src/web
# Dependencies on their own layer: an edit to a component reinstalls nothing.
COPY web/package.json web/bun.lock ./
RUN bun install --frozen-lockfile
COPY crates/compositor/fonts/ /src/crates/compositor/fonts/
COPY crates/render/src/page/shipped/kit.js /src/crates/render/src/page/shipped/kit.js
COPY web/ ./
RUN bun run build

FROM nginx:1.30-alpine
COPY deploy/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /src/web/dist /usr/share/nginx/html
