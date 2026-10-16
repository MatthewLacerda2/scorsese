// Every line ending in `// flagged` must be reported, and no other line.
const arbitrary = "bg-[#07070a] p-2"; // flagged
const short = "text-[#fff]"; // flagged
const style = { color: "#f4efe6" }; // flagged
const eight = { background: "#f4efe680" }; // flagged
const fn = { color: "rgb(244 239 230 / 0.6)" }; // flagged
const hsl = `hsl(${Math.round(120)} 70% 45%)`; // flagged
const oklch = { fill: "oklch(0.7 0.1 200)" }; // flagged
const palette = "bg-sky-500"; // flagged
const variant = "hover:text-zinc-400 p-1"; // flagged
const shade = "dark:border-red-600/50"; // flagged
const plain = "ring-white"; // flagged
const gradient = "from-black to-neutral-900"; // flagged
const fixed = `size-2 ${plain} bg-amber-500`; // flagged
const attr = <rect fill="#009b3a" />; // flagged
const cls = <div className="bg-black/30" />; // flagged
