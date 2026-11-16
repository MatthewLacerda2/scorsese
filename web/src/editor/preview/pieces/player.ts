// A preview played in pieces (#1448), the way a video site plays a video:
// a few seconds at a time from the playhead forward, each playable the
// moment it lands.
//
// **Media Source Extensions, not a playlist.** The server renders each
// piece as a fragmented MP4 — picture, and FLAC sound whenever the cut has
// any (`scorsese_render::preview::fragment`) — and this appends them to one
// buffer of a `MediaSource` the video element plays, each placed at its own
// time on the timeline. FLAC has no encoder priming, so the sound runs on
// across every join without a click. A growing HLS playlist would have
// needed a playlist route, a segment naming scheme and, outside Safari, a
// player library, for the same fragmented files; MSE needs none of that.
//
// One piece is asked for at a time, the next one missing from the playhead
// forward (`plan.ts`): previews render one at a time on the server, so asking
// for more at once would only queue them. A piece nobody wants any more — the
// playhead moved away, the player was paused a while — is cancelled, and a
// cancelled piece is never shown as a failure (#1439).

import type { Fps, RenderView } from "@/api";
import type { JobView } from "@/api/events";
import { toFrames, toSeconds } from "../../timeline/time";
import { aheadFor, nextPiece, type Piece, pieceLength, stillLoaded } from "./plan";

/** What asking for a piece answers: made or on its way, and whether the cut has sound. */
export interface Asked {
  render: RenderView | null;
  job: JobView | null;
  /** Whether every piece of this cut carries sound — known before the first is made. */
  audible?: boolean;
}

/** How the player reaches the server: `ask` for a piece, fetch its `bytes`, `cancel` a job. */
export interface Server {
  ask: (range: string) => Promise<Asked>;
  bytes: (render: RenderView) => Promise<ArrayBuffer>;
  cancel: (job: number) => void;
}

/** The codecs a piece is in: H.264 picture, and FLAC sound when the cut has any. A browser checks the family, not the level. */
const WITH_SOUND = 'video/mp4; codecs="avc1.640028,flac"';
const PICTURE_ONLY = 'video/mp4; codecs="avc1.640028"';

/** How often a piece on its way is asked about, in case no event arrives. */
const POLL_MS = 2000;

/** How many jobs one piece may take before it is given up on: a failure seen only by polling queues another. */
const ATTEMPTS = 3;

/** Whether this browser can play a preview in pieces at all. */
export function canPlayPieces(): boolean {
  return (
    typeof MediaSource !== "undefined" &&
    MediaSource.isTypeSupported(PICTURE_ONLY) &&
    MediaSource.isTypeSupported(WITH_SOUND)
  );
}

/** The buffered stretches of `element`, as `[start, end]` seconds. */
export function stretches(element: HTMLVideoElement): [number, number][] {
  const { buffered } = element;
  return Array.from({ length: buffered.length }, (_, at) => [buffered.start(at), buffered.end(at)]);
}

/** Thrown inside a piece's ask once nobody wants it. */
class Dropped extends Error {}

export class PiecePlayer {
  private readonly source = new MediaSource();
  readonly url = URL.createObjectURL(this.source);
  private buffer: SourceBuffer | null = null;
  private audible: boolean | null = null;
  private readonly opened: Promise<void>;
  private had = new Set<number>();
  private asking: { piece: Piece; job: number | null; dropped: boolean } | null = null;
  private readonly waiting = new Map<number, (state: string) => void>();
  private appending: Promise<void> = Promise.resolve();
  private lastTook: number | null = null;
  private at = 0;
  private idle = false;
  private gone = false;
  failed = false;

  constructor(
    private readonly element: HTMLVideoElement,
    private readonly server: Server,
    private readonly total: number,
    private readonly fps: Fps,
    private readonly changed: () => void,
  ) {
    element.src = this.url;
    this.opened = this.open().catch((error) => this.fail(error));
  }

  /** Play was pressed at `frame`: load from there, about ten seconds ahead. */
  play(frame: number) {
    this.idle = false;
    this.seek(frame);
  }

  /** The playhead is at `frame`: keep what is wanted from there, drop the rest. */
  seek(frame: number) {
    this.at = frame;
    const asking = this.asking;
    const wanted = this.next();
    if (asking && wanted && wanted.index !== asking.piece.index) this.drop(asking);
    this.pump();
  }

  /** Playing: follow the element's clock. */
  tick() {
    this.at = toFrames(this.element.currentTime, this.fps);
    this.pump();
  }

  /** Paused a while: stop making what may never be watched. */
  rest() {
    this.idle = true;
    if (this.asking) this.drop(this.asking);
  }

  /** Job `id` changed state on the event stream: wake whatever waits on it. */
  nudge(id: number, state: string) {
    this.waiting.get(id)?.(state);
  }

  /** Done with: nothing more is asked, and what is on its way is cancelled. */
  dispose() {
    this.gone = true;
    if (this.asking) this.drop(this.asking);
    for (const wake of this.waiting.values()) wake("cancelled");
    this.element.removeAttribute("src");
    this.element.load();
    URL.revokeObjectURL(this.url);
  }

  private async open() {
    await new Promise((opened) =>
      this.source.addEventListener("sourceopen", opened, { once: true }),
    );
  }

  /** The one buffer, made on the first piece, once the server has said whether the cut has sound. */
  private bufferFor(audible: boolean): SourceBuffer {
    if (!this.buffer) {
      this.buffer = this.source.addSourceBuffer(audible ? WITH_SOUND : PICTURE_ONLY);
      this.source.duration = toSeconds(this.total, this.fps);
    }
    return this.buffer;
  }

  /** The next piece wanted from the playhead, counting only what is still loaded. */
  private next() {
    this.had = stillLoaded(this.had, stretches(this.element), this.total, this.fps);
    return nextPiece(this.at, this.total, this.fps, this.had, aheadFor(this.lastTook));
  }

  /** Ask for the next piece wanted, if none is on its way. */
  private pump() {
    if (this.asking || this.idle || this.gone || this.failed) return;
    const piece = this.next();
    if (!piece) return this.finishIfWhole();
    const asking = { piece, job: null as number | null, dropped: false };
    this.asking = asking;
    const began = performance.now();
    const ask = () => this.server.ask(piece.range);
    void this.fetched(
      ask,
      () => asking.dropped || this.gone,
      (job) => (asking.job = job),
    )
      .then(async (bytes) => {
        await this.opened;
        if (asking.dropped || this.gone || this.audible === null) return;
        await this.append(this.bufferFor(this.audible), bytes, toSeconds(piece.start, this.fps));
        this.had.add(piece.index);
        this.lastTook = (performance.now() - began) / 1000;
      })
      .catch((error) => {
        if (!(error instanceof Dropped)) this.fail(error);
      })
      .finally(() => {
        if (this.asking === asking) this.asking = null;
        this.changed();
        this.pump();
      });
  }

  /** Stop wanting a piece: its job is cancelled and whatever it brings is ignored. */
  private drop(asking: NonNullable<PiecePlayer["asking"]>) {
    asking.dropped = true;
    if (asking.job !== null) {
      this.server.cancel(asking.job);
      this.waiting.get(asking.job)?.("cancelled");
    }
    if (this.asking === asking) this.asking = null;
  }

  /**
   * The bytes of one piece, once the server has made them — asking again as
   * its job moves. Throws `Dropped` once `dropped()` says so.
   */
  private async fetched(
    ask: () => Promise<Asked>,
    dropped: () => boolean,
    onJob: (job: number) => void = () => {},
  ): Promise<ArrayBuffer> {
    const jobs = new Set<number>();
    for (;;) {
      const asked = await ask();
      if (dropped()) throw new Dropped();
      if (asked.audible !== undefined) this.audible ??= asked.audible;
      if (asked.render) return this.server.bytes(asked.render);
      if (!asked.job) throw new Error("preview piece: no render and no job");
      jobs.add(asked.job.id);
      if (jobs.size > ATTEMPTS) throw new Error("preview piece: its jobs keep ending unmade");
      onJob(asked.job.id);
      const state = await this.settled(asked.job.id);
      if (dropped()) throw new Dropped();
      if (state === "failed" || state === "stuck") throw new Error(`preview job ${state}`);
    }
  }

  /** Until job `id` ends on the event stream, or a poll is due: its state then, if known. */
  private settled(id: number): Promise<string> {
    return new Promise((resolve) => {
      const wake = (state: string) => {
        if (state === "waiting" || state === "running") return;
        clearTimeout(timer);
        this.waiting.delete(id);
        resolve(state);
      };
      const timer = setTimeout(() => {
        this.waiting.delete(id);
        resolve("polled");
      }, POLL_MS);
      this.waiting.set(id, wake);
    });
  }

  /** Appends `bytes` to `buffer` at `offset` seconds, one append at a time. */
  private append(buffer: SourceBuffer, bytes: ArrayBuffer, offset: number): Promise<void> {
    const next = this.appending.then(
      () =>
        new Promise<void>((done, failed) => {
          if (this.gone) return done();
          buffer.timestampOffset = offset;
          buffer.addEventListener("updateend", () => done(), { once: true });
          buffer.addEventListener("error", () => failed(new Error("append failed")), {
            once: true,
          });
          buffer.appendBuffer(bytes);
        }),
    );
    this.appending = next.catch(() => {});
    return next;
  }

  /** Every piece is in: say so, so the element ends where the cut does. */
  private finishIfWhole() {
    const pieces = Math.ceil(this.total / pieceLength(this.fps));
    if (this.had.size < pieces || this.source.readyState !== "open") return;
    void this.appending.then(() => {
      if (this.source.readyState === "open" && !this.gone) this.source.endOfStream();
    });
  }

  private fail(error: unknown) {
    if (this.gone) return;
    console.error("preview: playing in pieces failed", error);
    this.failed = true;
    this.changed();
  }
}
