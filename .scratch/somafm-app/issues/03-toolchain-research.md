# 03 Toolchain: Vite+, Tauri 2, Svelte, Pico.css

Type: research
Status: resolved
Blocked by:

## Question

What is Vite+ (VoidZero) today, is it stable, and how does it scaffold/integrate with Tauri 2 and Svelte 5 + TypeScript (dev server, build, lint/test, tauri.conf devUrl/frontendDist)? Any known issues? Compare Pico.css against similarly lightweight options (e.g. Simple.css, Open Props, Water.css) for a Svelte app with a card grid and hover overlays, dark mode, and theming. Recommend one with reasons.

## Answer

Use the standard `create-tauri-app` Svelte + TypeScript template on plain Vite. Vite+ is dropped: it is beta with no documented Tauri 2 or Svelte integration. Decided by the user after the research (findings on local branch `research/toolchain`, `research/toolchain.md`). CSS framework: research recommends Open Props with scoped Svelte CSS (Pico.css is archived at v2.1.1); final CSS pick is still to confirm with the user.
