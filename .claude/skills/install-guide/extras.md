# Extras that run in Docker

Part of `install-guide`, step 8. Both need Docker running (`docker version`
answers with a *Server* section). Each is free and local, and each is offered
on its own: install only what the user says yes to.

## Kokoro: free local voices

The `kokoro-fastapi` server, which speaks an OpenAI-style
`/v1/audio/speech`. **scorsese does not call it and its compose stack does not
include it**: you make a line with it and import the file.

Check: `docker ps --filter name=kokoro` lists a running container. Otherwise:

```sh
docker run --detach --name kokoro --restart unless-stopped \
  -p 127.0.0.1:8880:8880 ghcr.io/remsky/kokoro-fastapi-cpu:v0.9.2
```

On a machine with an NVIDIA GPU and its container toolkit, the GPU image is
faster: `--gpus all` and `ghcr.io/remsky/kokoro-fastapi-gpu:v0.9.2` instead.
The first run downloads the image (a few GB); after that it starts in seconds
and comes back after a reboot until `docker rm -f kokoro`.

Check it speaks, then bring the line into a project:

```sh
curl -sS http://127.0.0.1:8880/v1/audio/speech \
  -H 'Content-Type: application/json' \
  -d '{"model":"kokoro","input":"Hello from scorsese.","voice":"af_heart","response_format":"mp3"}' \
  -o hello.mp3
scorsese import hello.mp3 --kind audio --project <the project>.scor
```

`curl http://127.0.0.1:8880/v1/audio/voices` lists the voices.

## Whisper: local transcription

The server `scorsese transcribe` reaches, for captioning real footage. Only
this step needs it; when it is not running, `transcribe` says how to start it.

Check: `curl http://127.0.0.1:8970/health` answers `OK`. Otherwise, from the
checkout:

```sh
docker compose --file deploy/whisper/compose.yaml up --detach
```

The first run builds the image and bakes in the Whisper large-v3-turbo model
(about 1.6 GB), so it needs the network and a few minutes. After that it
starts in seconds, runs offline and comes back after a reboot until
`docker compose --file deploy/whisper/compose.yaml down`. It takes about a
minute of CPU per half-minute of speech and keeps around 1 GB of RAM.

It answers at `http://127.0.0.1:8970`, where the CLI, the MCP server and the
desktop app look by default. If port 8970 is taken, change the published port
in that compose file and set `SCORSESE_WHISPER_URL` (or `whisper_url` in the
settings file) to match; `scorsese settings` prints the address it resolved.
Then `scorsese transcribe <asset>` on a project with imported footage lists
the words heard.
