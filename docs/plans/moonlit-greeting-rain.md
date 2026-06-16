# Page Refactoring Plan: Collect, Like, Mine

## Context

Collect.vue and Like.vue use a flat, basic design (`#f5f5f5` background, simple white cards, emoji icons) that clashes with the blog's premium aesthetic (gradient backgrounds, glass-morphism, soft shadows, orange-accented design system). Mine.vue is closer but needs dark-theme consolidation and minor polish. Additionally, Collect.css and Like.css are 99% identical duplicates with dead light-theme rules referencing wrong class names (`.collect-page`/`.like-page` instead of `.simple-page`).

## Approach

1. **Create shared `SimplePage.css`** — one CSS file imported by both Collect.vue and Like.vue with `scoped`, replacing two duplicate files
2. **Apply blog design language** — gradient backgrounds, glass-morphism header/cards, soft colored shadows, orange accents, smooth transitions
3. **Self-contained light/dark themes** — each CSS file defines both themes inline, following HomeView.css precedent
4. **Clean up global.css** — remove dead selectors (`.collect-page`, `.like-page`), remove now-redundant dark-theme overrides that scoped CSS will handle

## Files to Create

- `src/styles/views/SimplePage.css` — shared stylesheet for Collect + Like

## Files to Modify

- `src/views/Collect.vue` — change CSS import to SimplePage.css
- `src/views/Like.vue` — change CSS import to SimplePage.css
- `src/styles/views/Mine.css` — add self-contained dark theme block at bottom
- `src/assets/styles/global.css` — remove dead `.collect-page`/`.like-page` selectors, defang conflicting `.simple-page` dark background (let scoped CSS win)

## Files to Delete

- `src/styles/views/Collect.css`
- `src/styles/views/Like.css`

## Design Specs (SimplePage.css)

**Page shell:** radial + linear gradient background (warm white in light, near-black in dark)
**Header:** sticky, glass-morphism (`backdrop-filter: blur(12px)`, semi-transparent bg), orange logo
**Article cards:** `border-radius: 22px`, glass-morphism, soft shadow, `translateY(-3px)` hover lift, orange-tinged border on hover
**Cover image:** `110×76px`, `border-radius: 14px`
**Tags:** pill-shaped (`border-radius: 999px`), orange tint background
**Tab bar:** glass-morphism, refined inactive/active colors (`#94a3b8` / `#e87722`)
**Empty state:** larger icon, proper typography hierarchy, gradient go button
**Dark theme:** full block using `html[data-theme='dark'] .simple-page`

## Design Specs (Mine.css additions)

Add `html[data-theme='dark']` block covering background, card backgrounds, text colors — matching existing dark values from global.css so the page is self-documenting.

## Global.css Cleanup

- Remove `.collect-page`, `.like-page` from the dark-theme selector list (lines 132-133) — these class names never exist in the DOM
- Remove `html[data-theme='dark'] .simple-page` background/color rule (line 139) — SimplePage.css handles it with higher specificity
- Remove `html[data-theme='dark'] .mine-page` background rule (lines 114-118) — Mine.css will handle it
- Keep generic fallback rules (`.header`, `.tab-bar`, `.empty-state`, text colors) as safety net for other pages

## Migration Order

1. Create `SimplePage.css`
2. Switch Collect.vue import → verify light theme
3. Switch Like.vue import → verify light theme
4. Add dark theme block to Mine.css
5. Clean up global.css dead selectors
6. Delete Collect.css, Like.css
7. Verify all three pages in both themes at mobile/tablet/desktop

## Verification

- Visit `/collect`, `/like`, `/mine` in light theme — confirm gradient backgrounds, glass cards, proper spacing
- Toggle to dark theme — confirm near-black backgrounds, visible text, proper contrast
- Test empty states (no articles / no likes)
- Test responsive at 375px, 768px (tab bar → sidebar), 1200px
- Check that tab-bar remains functional with all four navigation links
