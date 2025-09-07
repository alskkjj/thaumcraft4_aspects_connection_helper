-- sqlite statement
CREATE TABLE IF NOT EXISTS elements (
	name TEXT PRIMARY KEY,
	belongs_to_mod TEXT,
	base_value REAL NOT NULL DEFAULT 1.0
	);

CREATE TABLE IF NOT EXISTS recipes (
	name TEXT,
	component_a TEXT,
	component_b TEXT,
	FOREIGN KEY (name) REFERENCES elements(name)
		ON UPDATE CASCADE ON DELETE CASCADE,
	FOREIGN KEY (component_a) REFERENCES elements(name)
		ON UPDATE CASCADE ON DELETE CASCADE,
	FOREIGN KEY (component_b) REFERENCES elements(name)
		ON UPDATE CASCADE ON DELETE CASCADE
	);

CREATE TABLE IF NOT EXISTS elements_holding(
	name TEXT,
	num REAL NOT NULL DEFAULT 0.0,
	FOREIGN KEY (name) REFERENCES elements(name)
		ON UPDATE CASCADE ON DELETE CASCADE
	);
