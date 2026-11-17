"""Recover device-grid spans from Double Range/column point measurements.

Do not ceil a Single ShapeRange.Width read-back to predict Chart.Export pixels.
No tuned epsilon: a candidate must lie inside the IEEE-754 Double rounding bin.
This is a scope-specific evidence check, not a generic Chart.Export algorithm.
"""
from fractions import Fraction
import math

def device_grid_px(points, dpi):
    points, dpi = float(points), int(dpi)
    if not math.isfinite(points) or points < 0 or dpi <= 0:
        raise ValueError('Invalid device-grid measurement')
    if points == 0:
        return 0
    center = Fraction.from_float(points)
    half_ulp = Fraction.from_float(math.ulp(points)) / 2
    scale = Fraction(dpi, 72)
    candidate = round(center * scale)
    if not (center - half_ulp) * scale <= candidate <= (center + half_ulp) * scale:
        raise ValueError('Point measurement does not certify an integer device grid')
    return candidate

def device_span_px(start_points, end_points, dpi):
    a, b = float(start_points), float(end_points)
    if not math.isfinite(a) or not math.isfinite(b) or b < a:
        raise ValueError('Invalid device span')
    center = Fraction.from_float(b) - Fraction.from_float(a)
    error = (Fraction.from_float(math.ulp(a)) + Fraction.from_float(math.ulp(b))) / 2
    scale = Fraction(int(dpi), 72)
    candidate = round(center * scale)
    if not (center - error) * scale <= candidate <= (center + error) * scale:
        raise ValueError('Endpoints do not certify an integer device span')
    return candidate

def native_width_matches(png_width, column_points, dpi):
    return int(png_width) == sum(device_grid_px(v, dpi) for v in column_points)

def png_phys_compatible(ppm, dpi):
    # pHYs stores integer pixels/metre: its quantization is not a new display DPI.
    return abs(Fraction(int(ppm)) - Fraction(int(dpi) * 10000, 254)) <= Fraction(1, 2)


def rgba_equal(a, b):
    """Compare every decoded channel; RGBA difference alpha alone is insufficient."""
    return a.size == b.size and a.convert("RGBA").tobytes() == b.convert("RGBA").tobytes()
