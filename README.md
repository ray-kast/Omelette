# Omelette

Omelette is a word game I started working on out of sheer boredom — I spent so
much time playing a similar word game on my phone that I got fed up with the
word list it was using and decided to write a version where I could specify the
words.

Fair warning — at the time of writing this readme, ***the wordlist IS NOT
censored***.  I have plans to allow blacklisting words, but I have not gotten
there yet, so if you want to run the game as it currently is, play at your own
risk.

## `tools/gen-words`

If you have a word list and want to use it with Omelette, the source tree inside
`tools/gen-words` can be built and run to generate a JSON word list file from
a plain text file containing the words to use, separated by whitespace.

## `tools/scrape-words`

Also in the tools folder is `scrape-words`, a tool designed to generate a
plaintext word list from some body of text.  It supports collecting data from
different sources and performing frequency analysis on it — see
[its readme](tools/scrape-words/README.md) for more details.