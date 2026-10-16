# 01 Audio playback across webviews

Type: research
Status: resolved
Blocked by:

## Question

Can an HTML5 `<audio>` element in Tauri 2's webviews (WebKitGTK on Linux, WebView2 on Windows, WKWebView on macOS) reliably play SomaFM's Icecast/Shoutcast streams (MP3, AAC, AAC+, HE-AAC; HTTP vs HTTPS, redirects, ICY metadata, CORS, mixed-content, autoplay policy)? What are the known failure modes, especially on WebKitGTK/GStreamer codec packages? If not reliable, what is the best native Rust alternative (rodio/symphonia/other) and its cost? Also: what hooks exist for media keys, tray and playback surviving window hide?

## Answer

Findings on local branch `research/audio-playback` (`research/audio-playback.md`; agent-reported, not yet reviewed in detail). `<audio>` can play SomaFM MP3/AAC in all three webviews, but mixed-content rules can block HTTP streams, Linux needs GStreamer packages for AAC, and media keys/background playback are limited. Alternative: native Rust (`rodio` + `symphonia`, `souvlaki` for media keys). The choice itself is decided in ticket 04.
