DRIFT_WINDOW = 64
DRIFT_PASSES = 90000


def score_reading(value: int) -> float:
    series = drift_series(value)
    return rolling_variance(series)


def drift_series(value: int) -> list:
    series = []
    seed = (value * 2654435761) % 4294967296
    for index in range(DRIFT_WINDOW):
        seed = (seed * 1103515245 + 12345) % 2147483648
        series.append(((seed % 20000) / 1000.0) - 10.0 + (index % 7))
    return series


def rolling_variance(series: list) -> float:
    variance = 0.0
    for _ in range(DRIFT_PASSES):
        variance = weighted_sum(series)
    return variance


def weighted_sum(series: list) -> float:
    mean = 0.0
    for sample in series:
        mean += sample
    mean /= len(series)
    total = 0.0
    for index, sample in enumerate(series):
        deviation = sample - mean
        total += deviation * deviation * (1.0 + (index / DRIFT_WINDOW))
    return total / len(series)
