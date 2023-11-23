INSERT INTO benchmarks
(
    year,
    day,
    participant,
    input,
    language,
    mean,
    stddev,
    median,
    tuser,
    tsystem,
    tmin,
    tmax
)
VALUES
(
    2023,
    1,
    'foo',
    'input-foo',
    'rust',
    0.004257588765,
    0.0011029383502713557,
    0.00461589472,
    0.0023045092000000002,
    0.001848802599999999,
    0.0005581637200000001,
    0.00510043572
),
(
    2023,
    1,
    'bar',
    'input-foo',
    'python',
    0.027436020056363638,
    0.004180552442992965,
    0.02631698022,
    0.023764949090909072,
    0.0030002865454545453,
    0.02507253372,
    0.056105820720000005
),
(
    2023,
    1,
    'foo',
    'input-bar',
    'rust',
    0.003087302970000001,
    0.004180411810488489,
    0.00223354172,
    0.0017269318000000006,
    0.0008888657999999998,
    0.0005558197200000001,
    0.04379071272
),
(
    2023,
    1,
    'bar',
    'input-bar',
    'python',
    0.029818995374545458,
    0.010439184292486534,
    0.026307431720000003,
    0.023231230909090912,
    0.003937978909090909,
    0.024945726720000002,
    0.07402750672
)
;
