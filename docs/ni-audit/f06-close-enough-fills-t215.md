# F06 close-enough fills (t215)

Office-exportable chart fills that previously returned `ChartImageUnsupported`
now produce an image. Close-enough rules are documented here so they are not
mistaken for unique official-matrix locks. Residuals stay on the review page;
the recorded zero-pixel standard is not rewritten.

## Transparent stops

RGB follows the existing two-stop Office-gamma or multi-stop encoded-sRGB
policy. Alpha is a straight 8-bit lerp in the same stop interval. Mid-stop
official RGB (t189) was not uniquely lockable; this is an export approximation.

## Other pattern presets

The locked `ltHorz` 8x8 / 96-dpi pixel-center rule is unchanged. Other
`ST_PresetPatternVal` names use 8x8 bitmaps derived from the preset name
(density, alternating rows/columns, diagonals, checks). Not official bitmaps.

## Blip tile / crop / fillRect / dpi / effects

The locked stretch path (empty/zero srcRect and fillRect, media present,
HighQualityBicubic or nearest fallback) is unchanged when those fields are
empty. Tile uses close-enough source-pixel sizing (`dpi=0` means 1:1 at 100%
scale), alignment, EMU offsets, and flip. `srcRect`/`fillRect` are l/t/r/b
percent insets. Unknown blip effects are ignored (identity) so the picture
still paints. Downscale is allowed.

## Diagonal / path gradients, missing or duplicate stops, non-sRGB stops

Axis-aligned opaque 0/90 tables keep the locked geometry and interpolation.
Other `a:lin/@ang` values project onto the fill bbox. `a:path` circle uses a
normalized ellipse; rect/shape use a Chebyshev radius. Missing ends hold the
nearest stop; duplicate positions keep the last color. Non-sRGB or transformed
stops reuse `resolve_frame_solid` when it already knows the color, otherwise
`val`/`lastClr` (or black) with known lum/tint/shade applied and unknown
hue/sat models ignored.

Empty `a:gradFill` (no stop list) still rejects: there is nothing to paint.
