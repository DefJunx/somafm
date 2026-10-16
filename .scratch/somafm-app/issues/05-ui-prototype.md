# 05 Prototype: grid, hover play, player screen

Type: prototype
Status: resolved
Blocked by:

## Question

How should the station grid, tile hover overlay with play button, the player screen (stop, volume, now-playing, quality, favourite) and the active-station indicator on the grid look and behave? Build a rough throwaway UI using real feed fields from the data research, react to it with the user, and lock layout and interaction decisions.

## Answer

Prototype is on the local throwaway branch `prototype/ui-grid-player` (commit 9f5cb18, worktree `../somafm-prototype-ui`, app in `prototype-ui/`). It is never merged. The user compared three variants and picked **Variant A "Tile wall"** across the board.

- **Grid:** a uniform grid of square artwork tiles. Hovering a tile shows a centred play button.
- **Tile click:** clicking anywhere on a tile opens the player screen and starts playing that station. This overrides the prototype, where a click only opened the player.
- **Player screen:** big artwork on the left; on the right the station name, description, listeners, now-playing track, stop, volume and mute, quality (MP3 128 / AAC 128), and a favourite toggle. Back sits at the top left.
- **Active station on the grid:** the playing tile gets an accent ring and an animated equaliser badge. A slim persistent "now playing" bar at the bottom of the grid has stop and a jump to the player.
- **Favourites:** a filter chip row above the grid: All / Favourites / genres. Genres come from splitting the feed's pipe-separated `genre`.
- **CSS:** Open Props confirmed ("it feels good").

Note (2026-10-16): the sleep timer was later ruled out of scope; references to it were removed above.
