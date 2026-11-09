# hole-free counter sample

The counter of `sample_counter` with `??reset` replaced by the button M5-27's test uses: the
hole-free counter of M6-18a, the text of `COUNTER` in paka/src/llvm/views.rs. Release builds it,
so M7-11 approves and releases this copy; the store is keyed by path, so it has its own records.
Run with `paka examples`. Expected output is written by hand.

- Added: the examples, after the app. `dec_stops_at_zero` and the four UI examples are those of
  `sample_counter`; `reset_to_zero` now passes, its click on `#2` sending `Reset`.
  `reset_clears` is a second function example, added for this sample.
