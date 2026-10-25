// How far a file is on its way, as a ring over its thumbnail.

const RADIUS = 9;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

export function ProgressRing({ progress }: { progress: number }) {
  const left = CIRCUMFERENCE * (1 - Math.min(Math.max(progress, 0), 1));
  return (
    <svg
      viewBox="0 0 24 24"
      className="size-7 -rotate-90 text-primary"
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(progress * 100)}
    >
      <circle cx="12" cy="12" r={RADIUS} fill="none" strokeWidth="3" className="stroke-muted" />
      <circle
        cx="12"
        cy="12"
        r={RADIUS}
        fill="none"
        strokeWidth="3"
        strokeLinecap="round"
        stroke="currentColor"
        strokeDasharray={CIRCUMFERENCE}
        strokeDashoffset={left}
      />
    </svg>
  );
}
