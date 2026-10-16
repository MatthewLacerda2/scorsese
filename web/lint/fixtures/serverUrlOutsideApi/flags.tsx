// Every way a server path can be written outside src/api.
declare const label: string;
const id = 7;
export const stream = "/api/events"; // flagged
export const root = "/api"; // flagged
export const single = '/api/uploads'; // flagged
export const project = `/api/projects/${id}`; // flagged
export const joined = `/api${stream}`; // flagged
export const asked = "/api?x=1"; // flagged
export const absolute = "https://scorsese.example/api/me"; // flagged
export const local = "http://localhost:8080/api/health"; // flagged
export const relative = "//scorsese.example/api"; // flagged
export const link = <a href="/api/auth/google">{label}</a>; // flagged
