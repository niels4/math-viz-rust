# Decisions deferred to `wrap`

What the port leaves out, and the planner's calls the milestones took. `wrap`
consolidates this file at the end; milestones append their own sections.

## h8

- The wordmark links to the root page: math-viz2's root route (`#`, the template's placeholder card) is never in the port, which has one route (`#function-viewer`), so the wordmark is a hot region with the pointer cursor and a no-op click on both targets (h8-plane).

## h10

- The welcome screen is ported as its own route (h10-page; plan §3.7 Routes supersedes the h8 wordmark line above): `""` is the welcome page, `function-viewer` the viewer, any other word the welcome page.
- Not ported: the template's not-found page (`_not_found.page.tsx`, the `Router`'s fallback for unknown routes) and the dev theme-demo page (`dev/theme-demo.page.tsx`, a developer route): SDL has no use for either, and the port's router maps every unknown word to the welcome page.
- The native binary opens on the welcome page by default, as the browser's `/` does (the driver's ruling for the run, overturnable by the user); `--route function-viewer` (h10-t-route's) opens the viewer directly.

## fix-cursor

- The function selector's cells show the pointer cursor (fix-cursor). The
  user's deliberate departure from math-viz2 at this hash, asked for in so
  many words (2026-10-20 11:20). The reference measures the arrow there:
  `auto` in WebKit, `default` in Chromium and Firefox (the covering radio
  input's UA default); no `cursor` rule on the cells at 68e1c7b or at main.
- The settings menu's light/dark cells and theme rows keep the pointer
  cursor the port showed before fix-cursor (fix-cursor). claude0's
  consistency call in the user's place (2026-10-20): they are the same kind
  of control as the selector cells, and the user saw them with the pointer
  and said nothing, so the arrow would read as a regression. Overturnable:
  the reference measures `default` in all three engines (its labels carry
  no cursor rule), so matching it is a two-line revert in `settings.rs`.
