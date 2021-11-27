# Advent of digIT

digIT's extended Advent of Code leaderboard

This project is meant to provide extra categories for Advent of Code.

## Running locally

When developing the easiest way to test the server is to simply run
`docker-compose up --build`. The `Dockerfile` is designed to cache most of the
build, and subsequent builds will only take around a fifth of the original time.
Hopefully making it fast enough that you are comfortable with developing around
it.

To test the full functionality you will need to update the following environment
variables: `AOC_SESSION`, `GITHUB_CLIENT_ID`, and `GITHUB_CLIENT_SECRET`. Your
updated values should **not** be committed! See the [Setup for production](#setup-for-production)
for details about these values.

## Setup for production

For production use, mostly take inspiration (read copy) from the
[`docker-compose.yml`](./docker-compose.yml) file. Some notable environment
variables are described here.

### `GAMMA_OWNER_GROUP`

This is a comma separated list of super groups from Gamma that should have admin
rights.

### `AOC_SESSION`

The session cookie from signing in to [adventofcode.com](https://adventofcode.com/).
The cookie should last the entire of December, but if you start seeing Internal
Server errors when loading leaderboards it might have expired.

Note that the user whose session cookie is provided must be a part of the
private leaderboard. Also, they must not sign out or the cookie will expire.

### `GITHUB_CLIENT_ID` and `GITHUB_CLIENT_SECRET`

GitHub client id and secret are for fetching the language statistics from
participants GitHub repositories. Should be set to a client that chit controls.

### `LEADERBOARD_CACHE_TIME` (and others)

These variables control the number of seconds a leaderboard is cached in the
Redis DB before it is refetched/recalculated.

If the languages leaderboard is returning Internal Server errors it might be
because the free tiers number of requests has been exceeded for the day. In that
case you can up the cache time for it. (Or find some money, but I don't believe
in capitalism.)
