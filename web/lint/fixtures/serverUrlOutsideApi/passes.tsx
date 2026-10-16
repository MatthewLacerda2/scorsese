// Literals that only look like a server path, and the path named in a comment
// (`/api/events`), which is documentation, not a URL in use.
const id = 7;
export const word = "/apiary";
export const nested = "/projects/api";
export const page = `/projects/${id}/edit`;
export const site = "https://scorsese.example/library";
export const text = "the /api prefix";
export const link = <a href="/login">sign in</a>;
