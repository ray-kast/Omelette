# `gen-words`

`gen-words` is a tool used to convert a plaintext wordlist into a database
usable by the game.

## Usage

`gen-words` requires `diesel_cli` to be installed - if you don't have it, you
can use the following command to install it with SQLite support only:<br>
`cargo install diesel_cli --no-default-features --features sqlite`<br>
If you have problems installing `diesel_cli`, you can refer to Diesel's
[getting started guide](http://diesel.rs/guides/getting-started/).

Command-line usage:<br>
`./run.sh <wordlist>`

`wordlist` is the name of a plaintext wordlist to read from.  It must contain
items separated by newlines (words separated by spaces only will be counted as
one).

## Usage with `process-12dicts`

A plaintext wordlist can be generated from the data contained in `etc/12dicts`
and `etc/alt12dicts` by using the script `process-12dicts` in the `scripts`
folder:

`scripts/process-12dicts >p12d.log && ./run.sh etc/wordlist.txt`
