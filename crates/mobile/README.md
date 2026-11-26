# oag-mobile

What a phone needs around the windowed game: where its files live and the
event loop each mobile OS hands the app, and the hand-over of a finished file to the phone's
shared storage (`export`) and the rate of the display the app is on (`refresh`, Android). One crate
for every phone OS.

The entry points stay in `oag-game`, which no crate may depend on. No gameplay
crate depends on this one. See
[ADR-0065](../../docs/architecture/adr/0065-one-mobile-crate-for-every-phone-os.md).
