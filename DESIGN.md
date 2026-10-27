---
name: SomaFM Player
description: Radio as spectrum. A dark charcoal tuning rail where every Station is a line in its genre hue.
colors:
  ground: "#16171a"
  plate: "#22242a"
  well: "#1b1c20"
  hairline: "#33353b"
  bone: "#e6e1d6"
  dim-ink: "#918f8a"
  violet: "#8b5cff"
  indigo: "#5b6cff"
  cyan: "#33c6ff"
  green: "#7dff5a"
  orange: "#ff7a2a"
  red: "#ff3b3b"
  sodium: "#ffc83a"
typography:
  display:
    fontFamily: "Schibsted Grotesk Variable, Schibsted Grotesk, ui-sans-serif, system-ui, sans-serif"
    fontSize: "40px"
    fontWeight: 700
    lineHeight: "48px"
    letterSpacing: "-0.02em"
  body:
    fontFamily: "Schibsted Grotesk Variable, Schibsted Grotesk, ui-sans-serif, system-ui, sans-serif"
    fontSize: "16px"
    fontWeight: 400
    lineHeight: "24px"
    letterSpacing: "0.04em"
  label:
    fontFamily: "Schibsted Grotesk Variable, Schibsted Grotesk, ui-sans-serif, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 500
    lineHeight: "16px"
    letterSpacing: "0.14em"
rounded:
  sm: "4px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "16px"
  lg: "24px"
  xl: "32px"
components:
  button:
    backgroundColor: "transparent"
    textColor: "{colors.bone}"
    rounded: "{rounded.sm}"
    padding: "0 16px"
    height: "32px"
  button-primary:
    backgroundColor: "{colors.bone}"
    textColor: "{colors.ground}"
    rounded: "{rounded.sm}"
    padding: "0 16px"
    height: "32px"
  field:
    backgroundColor: "transparent"
    textColor: "{colors.bone}"
    rounded: "{rounded.sm}"
    padding: "0 12px"
    height: "32px"
  plate:
    backgroundColor: "{colors.plate}"
    textColor: "{colors.bone}"
    rounded: "{rounded.sm}"
    padding: "24px"
  tile:
    backgroundColor: "{colors.well}"
    textColor: "{colors.bone}"
    rounded: "{rounded.sm}"
---

# Design System: SomaFM Player

## Overview

**Creative North Star: "The Tuning Rail"**

Radio as spectrum. Every Station is a line on one rail, and the line carries the Station's genre hue everywhere it appears: rail tick, Grid tile, Liked row, Now playing bar, Visualizer. State is told by line form (full, dashed, short, doubled) with a plain word beside it, never by colour alone. The ground is a dark charcoal continuum with faint horizontal banding; the world is dark-only.

The system is quiet and instrument-like. One grotesque, three type sizes, 1px hairlines, 4px corners, outline buttons, and a single inverted bone primary. Hierarchy comes from tracking, weight and ink rather than size or boxes. Colour belongs to the stations; the chrome stays charcoal and bone.

**Key Characteristics:**
- Lines, not tiles: a Station is a hue line before it is a card.
- Six genre hues plus one reserved accent (sodium for Liked).
- Three integer type sizes: 13 / 16 / 40.
- Outline controls at a fixed 32px; one inverted bone primary.
- Flat; depth by plate-on-ground tone and hairlines, with one static hue bloom on live elements.

## Colors

Charcoal and bone with six emission hues at full strength, each used only as a line, mark or glow.

### Primary
- **Bone** (`bone`): all body and heading ink, the focus ring default, selected-state borders, and the inverted primary button fill.

### Secondary
- **Violet, Indigo, Cyan, Green, Orange, Red** (`violet`, `indigo`, `cyan`, `green`, `orange`, `red`): the six genre emission hues. A Station takes the hue of its first genre (stable FNV-1a hash of the lower-cased genre name; no genre means violet). The hue is set as `--hue` on the element that carries it and is consumed by lines, the range fill, focus ring, the Visualizer and the live bloom.

### Tertiary
- **Sodium** (`sodium`): reserved for Liked only: filled heart, the doubled line on liked history rows, the Liked mark.

### Neutral
- **Ground** (`ground`): window background (with a 1px-in-4px 3% white banding), bar and sticky headers.
- **Plate** (`plate`): the Player plate, popovers, toast, hover rows.
- **Well** (`well`): art and tile backing.
- **Hairline** (`hairline`): borders, dividers, scrollbar, unfilled range track.
- **Dim ink** (`dim-ink`): secondary text, labels, placeholders. Chosen at this value for contrast on ground and plate. Also the fallback `--hue` while a Station is unknown.
- Soft hairline is white at 5% (row dividers); scrim is ground at 72% (art hover overlay).

### Named Rules
**The Reserved Sodium Rule.** Sodium means Liked and nothing else. It never marks a Station, a state or an action.
**The Hue Identity Rule.** A Station has one hue and it is identical on every surface. Never recolour a Station per view.
**The Not-By-Colour Rule.** Every hue-borne state is also carried by line form and a word.

## Typography

**Display, Body and Label Font:** Schibsted Grotesk Variable (self-hosted via @fontsource-variable, falling back to ui-sans-serif, system-ui)

**Character:** One grotesque for everything. Contrast comes from weight, tracking and ink, not from a second family or more sizes.

### Hierarchy
- **Display** (700, 40px, 48px line, -0.02em): Station name on the Player, view titles (Liked), empty-state titles. The only 40px use.
- **Body** (400 to 700, 16px, 24px line): descriptions (max 44ch), Station and track names in the bar, Grid and Liked rows, tile fallback names. Bold for Station names, 500 for track titles.
- **Label** (500, 13px, 16px line, 0.14em, uppercase, dim ink): h2/h3 section heads, the brand word, history head. State words (live, paused, status badge) use 0.1em tracking uppercase at 13px.
- **Small text** (400, 13px, 16px line, 0.04em): buttons, meta lines, artists, times, rail names (0.02em).

### Named Rules
**The Three Steps Rule.** Only 13, 16 and 40px exist. Do not add a size; adjust weight, tracking or ink instead.
**The Tabular Where Aligned Rule.** Tabular numerals only on times and counts that stack in a column.

## Layout

Operate-style app shell. A 48px top bar (muted tracked brand, Back or Stations, Liked, settings icon right) and a 56px fixed Now playing bar bracket every screen. The Player is a two-column grid: a 220px tuning rail plus 36px gap, then the plate; the rail runs the window height, ticks right-aligned, and the live Station's tick extends across the gap into the plate. Grid is an auto-fill grid of min 160px tiles with 24px/16px gaps; Liked is a single 880px max column with sticky header and day heads, 48px rows.

Spacing rhythm is 4/8/16/24/32. Controls and history rows are 32px; Station rail targets are 24px high (44px on coarse pointers) around 16px labels. Responsive: at 720px and below the rail collapses to 40px of ticks only (names appear in a plate tooltip on hover/focus, remaining in the accessible name), the brand hides and padding drops to 16px; at 600px the bar's track line hides; at 560px Back becomes icon-only. The window works down to 480 by 520.

## Elevation & Depth

Flat. Depth is tonal (ground, plate, well) plus 1px hairlines. There are no drop shadows on surfaces. The single exception is a static 35% hue bloom (`drop-shadow(0 0 4px` Station hue at 35%`)`) on live elements only: the live rail line, the live plate edge, and the Visualizer canvas. It is never animated and never on non-live elements.

### Named Rules
**The Live-Only Bloom Rule.** The hue bloom marks live state and nothing else.
**The Flat Rule.** Nothing else glows, lifts or casts a shadow.

## Shapes

4px radius on buttons, fields, tiles, plates, popovers and toast; art thumbnails in rows are square (0). Line language: lines are 1px by default (rail ticks, borders, liked and Station row marks, Visualizer bars). The only line thicker than 1px is the live 2px line: live rail tick, plate edge, history live marker, status dot, active-chip underline, range fill, Grid hue rule (2px, 24px wide, full width when active). Line forms: full = live, dashed (4px on, 3px off) = reconnecting, short (6px) = error, doubled sodium = liked.

## Components

### Buttons
- **Shape:** 4px radius, 32px high, 16px side padding, 1px hairline border, transparent fill, ink text with 0.04em tracking.
- **Hover:** border moves to dim ink. **Pressed/current:** border becomes bone. **Focus:** 2px outline in the Station hue (or bone), 2px offset. **Disabled:** 50% opacity.
- **Primary:** inverted bone fill, ground text, 700, only for the transport Play (Stop stays outline). Hover brightens 8%.
- **Icon button:** 32px square, no padding; ghost variants use a transparent border that shows hairline on hover.

### Tuning Rail
Right-aligned 16px labels in dim ink sit in 24px-high Station targets (44px on coarse pointers), with a 20px 1px hue tick centered on each target; hover lifts text to bone. The loaded Station is bold bone with a 2px tick extending across the gap, bloomed, plus a state word (live, paused, connecting, reconnecting, error). Width transition 0.3s.

### Chips
Outline buttons; selected is bold with a 2px hue underline inset 8px. Filter chips in Grid.

### Inputs / Fields
Hairline 1px border, 4px radius, 32px high, transparent fill, dim placeholder. Range is a 2px line filled in the Station hue over hairline with a 2px by 12px bone tick as thumb. Checkboxes use bone accent.

### Grid tile
Square well with 1px hairline, 4px radius; a 2px hue rule sits above it (24px, full width when active); hover border takes the hue, active border is bone. Name 16/700, genre 13 dim.

### Liked row and History row
Rows have a 1px hue line at the left edge. Liked rows show a sodium filled heart; history rows that are liked carry a doubled 1px sodium line (two 1px lines with a 1px gap) at the left. The live history row has a 2px hue marker and, if liked, the doubled line steps 5px inward. Favourite Station uses a star (filled when set), never a heart.

### Status badge and Now playing bar
Status badge: a 16px by 2px line (pulsing at 1.4s, still in dim ink when stopped, dashed when reconnecting) or a warning icon on error, with an uppercase tracked state word. The bar carries a 32px thumb with a 2px hue foot, bold Station, dim track line, heart, status badge and the action button.

### Icons
Inline SVG, 16px, 1.5px stroke, round joins, drawn in `currentColor`.

## Do's and Don'ts

### Do:
- **Do** carry a Station's hue as `--hue` on its element and draw it as a line, tick, fill or ring.
- **Do** keep every line 1px except the live 2px line.
- **Do** pair every hue-borne state with a line form and a word.
- **Do** reserve sodium for Liked, and use a star for favourite Station.
- **Do** keep dim text at #918f8a or lighter on ground and plate.
- **Do** use only 13, 16 and 40px; vary weight, tracking and ink.
- **Do** honour reduced motion (durations to 0, pulses and shimmer off).

### Don't:
- **Don't** use a uniform tile grid with a play-button accent and a separate player card.
- **Don't** colour a Station differently between surfaces, or use sodium for anything but Liked.
- **Don't** add shadows, gradients or glow other than the 35% static live bloom.
- **Don't** add a second typeface or a fourth size.
- **Don't** rely on colour alone to show state.
