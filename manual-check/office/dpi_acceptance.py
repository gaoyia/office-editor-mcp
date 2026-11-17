"""Explicit, scope-local t127 DPI acceptance gates; raw size/pixel results are diagnostic.

A certificate approves only its named scope, never its parent image.  Evidence must
bind identical source bytes and actual renderer identity.  Neither same size nor
sizeDiffer can infer semantic correctness or a DPI-only cause.
"""
PASS = "passed_dpi_only"
DIFFERENCE = "implementation_difference"
NI = "not_implemented"
PENDING = "insufficient_evidence"

def classify(*, scope=None, source_verified=False, renderer_verified=False,
             semantic_verified=False, font_verified=False, dpi_verified=False,
             unresolved=(), implementation=(), ni=False, raw_strict=""):
    """Return an evidence category, deliberately independent of raw_strict.

    `font_verified` means the effective font relevant to this scope, not merely
    equal PNG widths. Unresolved alpha/position/endpoint differences forbid a
    whole-image DPI waiver, but need not invalidate a separately named geometry
    or scalar certificate.
    """
    if ni:
        return NI
    if implementation:
        return DIFFERENCE
    if not scope or not (source_verified and renderer_verified and semantic_verified
            and font_verified and dpi_verified) or unresolved:
        return PENDING
    return PASS


def labels_between_axis_and_plot_bottom(axis_y, plot_bottom_y, ink_top, ink_bottom):
    """Native coordinate relation only; does not infer COM/PNG coordinate origins."""
    return axis_y < ink_top < ink_bottom < plot_bottom_y
