"""Native spark evidence only: ink support is not a certificate of endpoint geometry."""
from decimal import Decimal
from PIL import Image, ImageChops


def ink_bounds(image):
    rgb = image.convert('RGB')
    return ImageChops.difference(rgb, Image.new('RGB', rgb.size, 'white')).getbbox()


def ink_profile(image):
    rgb = image.convert('RGB')
    columns = [0] * rgb.width
    rows = [0] * rgb.height
    for y in range(rgb.height):
        for x in range(rgb.width):
            if rgb.getpixel((x, y)) != (255, 255, 255):
                columns[x] += 1
                rows[y] += 1
    return {'columns': columns, 'rows': rows}


def edge_crop_preserves_ink(raw, crop):
    """Only certify the historical origin-zero, last-column/last-row crop."""
    raw = raw.convert('RGB')
    crop = crop.convert('RGB')
    if raw.size != (crop.width + 1, crop.height + 1):
        return False
    if raw.crop((0, 0, crop.width, crop.height)).tobytes() != crop.tobytes():
        return False
    return all(raw.getpixel((raw.width-1, y)) == (255, 255, 255) for y in range(raw.height)) and all(raw.getpixel((x, raw.height-1)) == (255, 255, 255) for x in range(raw.width))


def line_weight_pixels(points, dpi):
    points, dpi = Decimal(str(points)), Decimal(str(dpi))
    if not points.is_finite() or not dpi.is_finite() or points <= 0 or dpi <= 0:
        raise ValueError('positive finite line weight and DPI required')
    return points * dpi / 72


def same_native_pixels(a, b):
    return a.size == b.size and a.convert('RGBA').tobytes() == b.convert('RGBA').tobytes()


def kernel_max_coverage(weight_pixels, fringe_pixels):
    """Analytical upper bound for the audited clamp(weight/2+fringe-distance) kernel.

    Not a replacement renderer, Office rule, or permission to change fixed-96 output.
    """
    weight, fringe = Decimal(str(weight_pixels)), Decimal(str(fringe_pixels))
    if not weight.is_finite() or not fringe.is_finite() or weight <= 0 or fringe < 0:
        raise ValueError('finite positive weight and nonnegative fringe required')
    return max(Decimal(0), min(Decimal(1), weight / 2 + fringe))


def normal_box_max_coverage(weight_pixels):
    """Current unit-normal interval-overlap maximum, not a diagonal area oracle."""
    weight = Decimal(str(weight_pixels))
    if not weight.is_finite() or weight <= 0:
        raise ValueError('positive finite weight required')
    return min(weight, Decimal(1))
