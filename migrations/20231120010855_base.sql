-- Benchmark records
CREATE TABLE benchmarks (
    id          integer GENERATED BY DEFAULT AS IDENTITY (START WITH 1000) PRIMARY KEY,
    year        integer NOT NULL,
    day         integer NOT NULL,
    input       varchar(256) NOT NULL,
    participant varchar(256) NOT NULL,
    language    varchar(256) NOT NULL,
    mean        double precision NOT NULL,
    stddev      double precision NOT NULL,
    median      double precision NOT NULL,
    tuser       double precision NOT NULL,
    tsystem     double precision NOT NULL,
    tmin        double precision NOT NULL,
    tmax        double precision NOT NULL,

    CONSTRAINT single_entry UNIQUE (year, day, input, participant)
);

CREATE INDEX benchmarks_participant_idx ON benchmarks (participant);

-- Summaries
CREATE TABLE summaries (
    year        integer NOT NULL,
    participant varchar(256) NOT NULL,
    language    varchar(256) NOT NULL,
    day_1       double precision,
    day_2       double precision,
    day_3       double precision,
    day_4       double precision,
    day_5       double precision,
    day_6       double precision,
    day_7       double precision,
    day_8       double precision,
    day_9       double precision,
    day_10      double precision,
    day_11      double precision,
    day_12      double precision,
    day_13      double precision,
    day_14      double precision,
    day_15      double precision,
    day_16      double precision,
    day_17      double precision,
    day_18      double precision,
    day_19      double precision,
    day_20      double precision,
    day_21      double precision,
    day_22      double precision,
    day_23      double precision,
    day_24      double precision,
    day_25      double precision,
    total       double precision,
    PRIMARY KEY (year, participant)
);

CREATE INDEX summaries_participant_idx ON summaries (participant);
CREATE INDEX summaries_language_idx ON summaries (language);
