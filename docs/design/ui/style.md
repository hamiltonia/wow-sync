# UI design: visual system

Part of the [UI design](README.md). Values are logical pixels (GPUI `px`),
which GPUI scales by the window's scale factor. Colours are sRGB hex.

## Theme selection

- Default: **follow the system** light/dark setting (D-19), read with GPUI
  `window.appearance()` and updated live with `observe_window_appearance`
  (`Light`/`VibrantLight` → light, `Dark`/`VibrantDark` → dark). Whether
  Windows reports changes live at the pinned GPUI is to be verified (Q-35).
- App settings can override to Light or Dark (Q-34).
- Tokens are the only colours used; no screen hard-codes a colour (the slice's
  `BG`, `PANEL`, `MUTED` constants are replaced by tokens).
- No transparency, blur or vibrancy: plain fills keep contrast predictable.

## Colour tokens

### Surfaces

| Token | Light | Dark | Use |
|---|---|---|---|
| `bg` | `#F5F6F8` | `#14171C` | window background behind panels |
| `surface` | `#FFFFFF` | `#1D2128` | panels, cards, header |
| `sidebar` | `#EBEEF2` | `#191C22` | sidebar background |
| `selected` | `#DCE6F7` | `#26344D` | selected sidebar item, selected list row |
| `hover` | `#F0F3F7` | `#242932` | row/button hover (no text-contrast change) |
| `divider` | `#D5DAE1` | `#2C313A` | decorative separators (no contrast requirement) |
| `control-border` | `#7D8799` | `#6B7585` | borders of buttons, toggles, checkboxes (≥ 3:1, WCAG 1.4.11) |

### Text and accents

Contrast ratios are WCAG 2.x relative-luminance ratios, computed for this
design (script in the [evidence](../../runs/run-003/evidence/R3-ui-design.md)).
**Every text token is ≥ 4.5:1 on every surface it can appear on**, including
the `selected` background.

Light theme:

| Token | Hex | on `bg` | on `surface` | on `sidebar` | on `selected` | min |
|---|---|---|---|---|---|---|
| `text` | `#1A1F29` | 15.27 | 16.51 | 14.19 | 13.13 | 13.13 |
| `text-muted` | `#4A5363` | 7.17 | 7.75 | 6.66 | 6.17 | 6.17 |
| `accent` (links, primary) | `#1D5BBF` | 5.88 | 6.36 | 5.47 | 5.06 | 5.06 |
| `ok` | `#17703A` | 5.69 | 6.15 | 5.28 | 4.89 | 4.89 |
| `working` | `#1D5BBF` | 5.88 | 6.36 | 5.47 | 5.06 | 5.06 |
| `waiting` | `#8A5300` | 5.85 | 6.33 | 5.44 | 5.03 | 5.03 |
| `help` | `#A64205` | 5.71 | 6.17 | 5.30 | 4.91 | 4.91 |
| `error` | `#B42318` | 6.08 | 6.57 | 5.65 | 5.23 | 5.23 |
| `unknown` | `#4A5363` | 7.17 | 7.75 | 6.66 | 6.17 | 6.17 |
| `paused` | `#6941C6` | 6.12 | 6.62 | 5.69 | 5.27 | 5.27 |

Dark theme:

| Token | Hex | on `bg` | on `surface` | on `sidebar` | on `selected` | min |
|---|---|---|---|---|---|---|
| `text` | `#E8EAEE` | 14.91 | 13.41 | 14.17 | 10.37 | 10.37 |
| `text-muted` | `#A7AFBB` | 8.12 | 7.30 | 7.71 | 5.65 | 5.65 |
| `accent` | `#8AB8FF` | 8.88 | 7.98 | 8.44 | 6.18 | 6.18 |
| `ok` | `#5CCB7F` | 8.81 | 7.92 | 8.37 | 6.13 | 6.13 |
| `working` | `#8AB8FF` | 8.88 | 7.98 | 8.44 | 6.18 | 6.18 |
| `waiting` | `#F2C14E` | 10.70 | 9.62 | 10.17 | 7.45 | 7.45 |
| `help` | `#FFA463` | 9.19 | 8.26 | 8.73 | 6.39 | 6.39 |
| `error` | `#FF8A80` | 7.87 | 7.07 | 7.48 | 5.47 | 5.47 |
| `unknown` | `#A7AFBB` | 8.12 | 7.30 | 7.71 | 5.65 | 5.65 |
| `paused` | `#C4A8FF` | 8.89 | 7.99 | 8.45 | 6.18 | 6.18 |

### Buttons, chips and badges

| Pair | Light | Ratio | Dark | Ratio |
|---|---|---|---|---|
| Primary button text on `accent` fill | `#FFFFFF` on `#1D5BBF` | 6.36 | `#0E1626` on `#8AB8FF` | 8.94 |
| Secondary button | `text` on `surface`, `control-border` | 16.51 | `text` on `surface` | 13.41 |
| Count badge (needs attention) | `#FFFFFF` on `help` `#A64205` | 6.17 | `#14171C` on `help` `#FFA463` | 9.19 |
| Error badge | `#FFFFFF` on `error` `#B42318` | 6.57 | `#14171C` on `error` `#FF8A80` | 7.87 |
| Disabled text (exempt; still legible) | `#8A94A6` on `surface` | 3.06 | `#6B7585` on `surface` | 3.47 |
| `control-border` vs `surface` / `bg` / `sidebar` | `#7D8799` | 3.62 / 3.35 / 3.11 | `#6B7585` | 3.47 / 3.86 / 3.66 |

Status chips use a tinted fill with the status colour as text. Measured
ratios (status text on its chip fill):

| Status | Light fill | Light ratio | Dark fill | Dark ratio |
|---|---|---|---|---|
| `ok` | `#E3F3E8` | 5.34 | `#1C3326` | 6.64 |
| `working` | `#E3ECFA` | 5.35 | `#1E2D47` | 6.82 |
| `waiting` | `#FBF0D9` | 5.60 | `#3A2F14` | 7.84 |
| `help` | `#FDEBDD` | 5.32 | `#3F2716` | 7.10 |
| `error` | `#FCE4E2` | 5.43 | `#432020` | 6.27 |
| `unknown` | `#ECEEF1` | 6.67 | `#2A2F37` | 6.08 |
| `paused` | `#EEE8FB` | 5.54 | `#2E2645` | 7.02 |

`text` on every chip fill is ≥ 13.6:1 (light) and ≥ 10.9:1 (dark), so chip
fills can also back body text in banners.

Banners use the chip fill as background, a 4 px left bar in the status
colour, the status icon, and `text` for the message.

## Status vocabulary

Every status is **icon + word + colour**; colour never stands alone (R-13
accessibility; colour-blind users).

| Status | Token | Icon (shape) | Words used |
|---|---|---|---|
| Up to date / success | `ok` | check mark | "Up to date", "Updated here", "Confirmed" |
| Working | `working` | two circular arrows | "Syncing…", "Updating…", "Sharing now…" |
| Waiting | `waiting` | hourglass | "Waiting…", "Update ready", "Waiting for Dropbox" |
| Needs you | `help` | exclamation in a triangle | "Needs your decision", "Needs help" |
| Error | `error` | cross in a circle | "Can't read…", "Could not…" |
| Unknown / not confirmed / not synced | `unknown` | clock (stale), dashed circle (not confirmed), slashed circle (not synced) | "Not confirmed yet", "Not heard from for…", "Not synced here" |
| Paused | `paused` | two vertical bars | "Paused" |

The four distinct sync facts (R-12) have distinct words and never share one:

| Fact | Words | Style |
|---|---|---|
| Published (this computer shared its state) | "Shared 12 min ago" | `text-muted`, no icon |
| Available here (target differs, data present) | "Update ready" | `working` |
| Applied here | "Updated here" | `ok` |
| Confirmed by another computer | "Confirmed by Laptop (report received 5 min ago)" | `ok` |

## Status icons

Icons are simple single-colour SVG paths authored in the repository (no new
icon-font or icon-pack dependency), drawn on a 16 × 16 grid with a 1.5 px
stroke, rendered with GPUI `svg()` from an embedded `AssetSource` and tinted
with the status token. Sizes: 16 px in lists and chips, 20 px in the sidebar,
24 px in the header. Whether GPUI `svg()` tinting works for these assets at the
pin is a prototype check; the fallback is the word alone, which is always
present anyway.

Tray icons: a base mark (a stylised "W" with two arrows) plus a status overlay
in the lower-right quadrant using the same shapes (check, arrows, hourglass,
exclamation, cross, two bars). Windows: ICO with 16, 20, 24, 32, 40 and 48 px
frames; macOS: a template image (monochrome, the system tints it) at 1× and 2×,
with the overlay shape carrying the meaning. Tray icons must be legible on light
and dark taskbars: the Windows variants use a 1 px contrasting outline.

## Focus ring

- 2 px solid ring, drawn 2 px outside the control (a 2 px gap in the
  surrounding surface colour), radius = control radius + 2.
- Light: `#1D5BBF` (≥ 5.06:1 against every surface, 6.36:1 against
  `surface`). The gap keeps the ring visible around the `accent`-filled primary
  button.
- Dark: `#FFD166` (≥ 8.67:1 against every surface), which keeps the slice's
  yellow ring.
- Visible on keyboard focus always. Mouse clicks may also show it; GPUI has no
  `:focus-visible` distinction, so the ring shows on any focus.
- Never removed, never only a colour change of the border.

## Spacing scale

Base unit 4 px: `space-1` 4, `space-2` 8, `space-3` 12, `space-4` 16,
`space-5` 20, `space-6` 24, `space-8` 32, `space-12` 48.

| Use | Value |
|---|---|
| Panel padding | 20 |
| Gap between panels | 16 |
| Content area padding | 24 (16 below 900 px window width) |
| Header height | 76 (headline row + context row), padding 16 / 24 |
| Sidebar width / item height / item padding | 220 / 40 / 12 horizontal |
| List row height (min) | 44 (rows grow with wrapped text) |
| Key/value row | 32 min, label column 160 |
| Button height / horizontal padding | 32 / 14; touch-friendly minimum target 32 × 32 |
| Gap between icon and label | 8 |
| Radius: panels / buttons / chips | 8 / 6 / 10 (pill) |

## Type scale

Font: the system UI font (Windows: Segoe UI Variable, falling back to Segoe UI;
macOS: the system font). GPUI's `.SystemUIFont` alias is used when it resolves;
otherwise name the family per OS. Numbers in tables use tabular figures if the
font provides them.

| Role | Size / line height | Weight | Use |
|---|---|---|---|
| `headline` | 22 / 28 | Semibold | header headline |
| `title` | 20 / 28 | Semibold | page title |
| `section` | 15 / 22 | Semibold | panel titles |
| `body` | 14 / 20 | Regular | text, rows, buttons |
| `body-strong` | 14 / 20 | Semibold | names in rows, key labels |
| `caption` | 12 / 16 | Regular | times, secondary lines, chip text |
| `mono` | 12 / 16 | Regular, monospace | Details only (hashes, IDs) |

Minimum text size anywhere is 12 px. Text wraps; no truncation of status
sentences. Add-on names may truncate with an ellipsis in list rows only, with
the full name in the accessible name and on the detail page.

## High-DPI and scaling rules

1. Author every size in logical px; never in physical pixels. GPUI multiplies
   by the window scale factor (100 %, 125 %, 150 %, 175 %, 200 %, and
   per-monitor changes on Windows).
2. No raster images inside the window; icons are SVG. Only the tray icon is
   raster, with the frame set above.
3. Hairlines are 1 logical px; never 0.5 px.
4. Layout must work from the 760 × 520 minimum at 100 % up to 4K at 200 %:
   flex layouts, wrapping text, scrolling content; no fixed heights on text
   containers.
5. When the window moves to a monitor with a different scale, the layout
   re-renders at the new scale without restarting (prototype check at 100 %
   and 150 % on Windows).
6. Windows "Make text bigger" (text scaling) is not known to be honoured by
   GPUI; the prototype records whether it is. If not, it is a listed
   limitation, not a silent omission (R-13).
7. Screenshots for owner review are taken at 100 % and 150 %, in light and
   dark.
