# counter sample

The counter of GENESIS.md section 4 (lines 201 to 234), run with `paka examples`. Expected output
is written by hand.

Differences from the text of GENESIS.md:

- None; the text is unchanged.
- Added: four UI examples (M5-23). `inc_once` is the one in docs/decisions/M5-06.md. Three pass;
  `reset_to_zero` clicks the `??reset` placeholder, so it is unfinished (M5-06 section 1.6) and
  the run exits 3. It passes once the placeholder becomes a button that sends a reset message.
