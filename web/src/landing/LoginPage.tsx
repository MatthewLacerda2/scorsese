// `/login` (#904): where the session guard sends a visitor, and old bookmarks
// point. `LandingPage.tsx` says what it does with the popup.

import { LandingPage } from "@/landing/LandingPage";

/** `/login`: the landing page with the popup open. */
export function LoginPage() {
  return <LandingPage signIn />;
}
