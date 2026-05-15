# QUIL_WORD_NAVIGATION_V1

## Goal
Add word-left/word-right cursor navigation. In-memory only.

## Functions
- `cursor_word_left()`: skip trailing spaces, then skip word chars
- `cursor_word_right()`: skip current word, then skip spaces

## Proof
3 moves: pos 11→8 (skip "ghi"), 8→4 (skip "def"), 4→8 (word-right)

## Safety
No kernel/ABI changes. Bounded buffer.
