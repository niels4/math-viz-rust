# Decisions deferred to `wrap`

What the port leaves out, and the planner's calls the milestones took. `wrap`
consolidates this file at the end; milestones append their own sections.

## h8

- The wordmark links to the root page: math-viz2's root route (`#`, the template's placeholder card) is never in the port, which has one route (`#function-viewer`), so the wordmark is a hot region with the pointer cursor and a no-op click on both targets (h8-plane).

## h10

- The welcome screen is ported as its own route (h10-page; plan §3.7 Routes supersedes the h8 wordmark line above): `""` is the welcome page, `function-viewer` the viewer, any other word the welcome page.
- Not ported: the template's not-found page (`_not_found.page.tsx`, the `Router`'s fallback for unknown routes) and the dev theme-demo page (`dev/theme-demo.page.tsx`, a developer route): SDL has no use for either, and the port's router maps every unknown word to the welcome page.
- The native binary opens on the welcome page by default, as the browser's `/` does (the driver's ruling for the run, overturnable by the user); `--route function-viewer` (h10-t-route's) opens the viewer directly.
