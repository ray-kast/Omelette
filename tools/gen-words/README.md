# `gen-words`

`gen-words` is a tool used to convert a plaintext wordlist into a database
usable by the game.

## Usage

Command-line usage:
`./run.sh <wordlist>`

`wordlist` is the name of a plaintext wordlist to read from.  It must contain
items separated by newlines (words separated by spaces only will be counted as
one).

## Usage with `process-12dicts`

A plaintext wordlist can be generated from the data contained in `etc/12dicts`
and `etc/alt12dicts` by using the script `process-12dicts` in the `scripts`
folder:

`scripts/process-12dicts && ./run.sh etc/wordlist.txt`
