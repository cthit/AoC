CREATE TABLE IF NOT EXISTS Users (
	cid TEXT NOT NULL,
	aoc_id TEXT NOT NULL,

	UNIQUE (aoc_id),

	PRIMARY KEY (cid)
);

CREATE TABLE IF NOT EXISTS Years (
	year INTEGER NOT NULL,
	leaderboard TEXT NOT NULL,

	PRIMARY KEY (year)
);

CREATE TABLE IF NOT EXISTS Participants (
	cid TEXT NOT NULL,
	year INTEGER NOT NULL,
	github TEXT,

	UNIQUE (cid, year),

	FOREIGN KEY (cid) REFERENCES Users(cid),
	FOREIGN KEY (year) REFERENCES Years(year)
);
