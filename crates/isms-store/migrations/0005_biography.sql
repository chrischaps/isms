-- S1.13: the one line of profile a player writes (GDD 10). Cosmetic and
-- reputational; it carries across societies and epochs, nothing material does.
ALTER TABLE accounts ADD COLUMN biography TEXT NOT NULL DEFAULT '';
