# 02 SomaFM data feeds

Type: research
Status: resolved
Blocked by:

## Question

What public feeds does SomaFM offer (channels.json, song history, playlists)? For each: URL, fields (title, description, image sizes, genre, listeners, stream URLs and formats/quality tiers), update cadence, CORS headers, how to get now-playing track, rate limits, and any terms of use or attribution requirements for third-party apps.

## Answer

Findings on local branch `research/somafm-data` (`research/somafm-data.md`). Single feed: `https://somafm.com/channels.json` (CORS enabled, ~5s cache): 51 channels with listeners, now-playing text, DJ, three image sizes (120 to 512 px), playlist URLs for MP3/AAC/AAC+ across tiers over three servers. No structured song-history API.

**Blocker (verified against https://somafm.com/contact/tos.html, updated Sep 4 2026):** SomaFM states "We can't grant permission for third-party SomaFM clients or applications, even noncommercial ones", is not approving new ones, and says streams, branding, artwork and metadata may not be used in standalone apps. Only a few older general-purpose tuner apps are excepted. This puts the whole destination in question; see ticket 06.
