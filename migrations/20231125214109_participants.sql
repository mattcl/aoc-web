-- Add new participants table
CREATE TABLE participants (
    year        integer NOT NULL,
    name        varchar(256) NOT NULL,
    repo        varchar(512) NOT NULL,
    language    varchar(256) NOT NULL,
    PRIMARY KEY (year, name)
);

CREATE INDEX participants_name_idx ON participants (name);
