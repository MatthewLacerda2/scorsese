# syntax=docker/dockerfile:1
#
# The backup sidecar: pg_dump and rclone, on a loop. Built on the database's own
# image so pg_dump is always the version of the server it dumps — a newer
# server than pg_dump is a dump that refuses to run, found out on the night it
# was needed.
FROM postgres:17-alpine
RUN apk add --no-cache rclone
COPY --chmod=755 backup backup-loop backup-fresh /usr/local/bin/
CMD ["backup-loop"]
