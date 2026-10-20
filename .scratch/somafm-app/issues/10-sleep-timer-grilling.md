# 10 Sleep timer UX

Type: grilling
Status: resolved
Blocked by: 

## Question

How does the Sleep timer work for the user? Decide: the preset durations and whether custom values are allowed, where it is set and shown (Player screen, bottom bar, tray menu entry), whether a manual Stop or a station change cancels it, behaviour while the window is hidden to the tray, and any fade-out on expiry. Already fixed: it runs in Rust on the wall clock, keeps running through reconnects and error, stops playback on expiry and never quits the app.

## Answer

Out of scope. The user does not want a sleep timer (2026-10-16). No timer anywhere: not in PlayerState, commands, the Player screen, the tray or tests.
