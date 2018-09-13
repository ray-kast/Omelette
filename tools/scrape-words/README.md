# `scrape-words`

`scrape-words` is a tool designed to collect data from a document or corpus of
your choice and convert it to a wordlist designed for use with `gen-words`.

## Usage

Command-line usage:
`scrape-words <source> <process>`

`source` can be one of the following:

- `reddit <parameters...>`: scrapes data from Reddit.
  See the section on scraping from Reddit for more details.
- `local <path>`: reads data from the file located at `path`.

`process` can be one of the following:

- `analyze <path>`: performs frequency analysis on the collected corpus and
  dumps the output to the file located at `path`.
- `dump <path>`: dumps the corpus as plaintext into the file located at `path`.
  Useful for caching scraped data for analyzing later.

## Analyzing a Local File

To analyze a local file, use the `local` source selector in conjunction with the
`analyze` process selector, like so:

```
$ scrape-words local ~/Documents/diary.txt analyze freq.log
```

## Scraping from Reddit

`scrape-words` can use Reddit's OAuth API to retrieve posts from Reddit.  Note
that this will require you to have the credentials of a valid Reddit bot.

I highly recommend you use the `reddit` source in conjunction with the `dump`
process specifier, as downloading the post data can take awhile due to Reddit's
API rate limit, so it's far more convenient to dump the raw text into a file
and analyze that later.

### Authentication

In order to use the Reddit API client, the file `etc/apikey.reddit.json` must be
created with the following contents:

```json
{
  "id": "<your Reddit app ID>",
  "secret": "<your Reddit app's client secret>"
}
```

If you don't have a Reddit app registered, you can create one
[here](https://www.reddit.com/prefs/apps).

**A quick note on privacy:** this script does not share any information about
your app or your account with anyone. It only tracks enough data to log itself
in again, and all the tracked data can be viewed in the file
`etc/apitok.reddit.json` (it's mainly just the OAuth tokens).

### Command-line Syntax

The syntax for the Reddit source is as follows:

`reddit <subreddit> <sort> <limit> <pretty-log>`

`subreddit` is the name of the subreddit you would like to scrape posts from
(without the r/ prefix).

`sort` selects how the posts are sorted and can be one of the following:

- `hot`: sort posts by Hot
- `new`: sort posts by New
- `rising`: sort posts by Rising
- `top(<time>)`: sort posts by Top for the given time range (for instance,
  `top(all)` sorts by top of all time)
- `controversial(<time>)`: sort posts by Controversial for the given time range

for `top` and `controversial`, the `time` parameter can be `hour`, `day`,
`week`, `month`, `year`, or `all`.

`limit` is the number of posts you would like to retrieve.  Note that this
script is subject to the ~1000 post limit imposed by Reddit, although I may look
into one of the workarounds for this in the future.

`pretty-log` is the name of a file into which the script will dump a
pretty-printed version of the posts it retrieved.