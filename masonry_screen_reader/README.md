# Masonry Screen reader

Screen reader debug output for Masonry accessibility trees.

This crate provides debugging tools to simulate screen reader behavior by generating
human-readable descriptions of accessibility tree updates.
It helps developers understand what information would be announced to screen reader
users as they interact with a Masonry application.

While this crate isn't trying to emulate a specific screen reader (yet), it aims to
emulate the *median* screen reader.
That means, among other things, that it shouldn't give users information that most
screen readers wouldn't give.
