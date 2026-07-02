# QUIL_LINE_STATS_V1

## Goal
Add bytes/lines/words/cursor stats markers.

## Functions
- `count_words()`: space-delimited word count
- `emit_text_stats()`: prints `[quil.text.stats] bytes=N lines=N words=N cursor=N`

## Proof
2 stats snapshots at cursor pos 5 and 12.

## Safety
No kernel/ABI changes. Bounded buffer scan.
