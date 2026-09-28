// A URL no page answers.

import { Link } from "react-router";

export function NotFound() {
  return (
    <p className="text-muted-foreground">
      There is nothing here.{" "}
      <Link to="/projects" className="underline">
        Go to your projects
      </Link>
    </p>
  );
}
