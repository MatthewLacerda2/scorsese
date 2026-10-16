// biome-ignore-all lint/plugin/oneComponentPerFile: a fixture showing a file-wide exception and its reason
// A file that keeps two exported components on purpose says so once, at the top.
export function Tabs() {
  return <div />;
}

export function Tab() {
  return <span />;
}
