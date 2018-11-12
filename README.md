# Omelette

Omelette is a word game I started working on out of sheer boredom — I spent so
much time playing a similar word game on my phone that I got fed up with the
word list it was using and decided to write a version where I could specify the
words.

**A note on usage:** The game does not come with a pre-generated word list —
before running it for the first time, you must generate one.  See [here](tools/gen-words/README.md)
for more details.

## Controls

Aside from basic text-editing controls for the word box, the following is a list
of controls for the game as it currently stands (assuming an xterm-like console):

| Key | Command |
|-:|:-|
| `Ctrl+D`    | Quit the application. |
| `Tab`       | Shuffle the remaining letters. |
| `Ctrl+Bksp` | Clear the word box. |
| `Esc`       | Forfeit the current level and pick a new word. |
| `Enter`     | Submit your guess. |
| `Shift+Tab` | Toggle auto-sorting the remaining letters alphabetically. |

## `tools/gen-words`

If you have a word list and want to use it with Omelette, the source tree inside
`tools/gen-words` can be built and run to generate a JSON word list file from
a plain text file containing the words to use, separated by newlines.

## `tools/scrape-words`

Also in the tools folder is `scrape-words`, a tool designed to generate a
plaintext word list from some body of text.  It supports collecting data from
different sources and performing frequency analysis on it — see
[its readme](tools/scrape-words/README.md) for more details.