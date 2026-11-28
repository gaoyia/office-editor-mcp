# F24 close-enough: 3D charts produce an image

Official F24 Chart.Export gold already existed for all five 3D wrappers
plus the 2D clustered-column control in
`C:\\Users\\AMD\\AppData\\Local\\Temp\\t169-f24-3d-20261121`
(Excel 16, window DPI 106, CLEAN_PUBLIC_OFFICE_EXPORT,
`chartExportReturned` true). Excel was not relaunched. The 2D control
already exported and is regenerated only. Residual pixels stay on the
review page. The recorded zero-pixel standard is not met and is not
rewritten as if it were.

## What now exports

These official-like wrappers must produce an image. NotImplemented /
ChartImageUnsupported is not acceptable for them:

- `bar3DChart` barDir=col (or omitted), grouping=clustered (or omitted),
  shape=box (or omitted)
- `line3DChart` grouping=standard (or omitted)
- `area3DChart` grouping=standard (or omitted)
- single-series `pie3DChart` (no axes)
- `surface3DChart` wireframe=0 (or omitted)

Projection uses explicit `c:view3D` `rotX` / `rotY` / `rAngAx` when
present. Official gold in this folder writes cartesian rotX=15, rotY=20,
rAngAx=0 (perspective on) and pie rotX=15, rotY=0, rAngAx=0, and omits
perspective / depthPercent / hPercent. Omitted view3D children use
ECMA-376 schema defaults (rotX/rotY 0, rAngAx true, perspective 30,
depthPercent/hPercent 100). The perspective division, box/ribbon/prism
and pie thickness, wall layout, simple face shading, and surface
major-unit value bands are close-enough approximations, not unique
official-matrix locks.

Title and legend stay in 2D screen space. Official pie3D writes wrapper dLbls with all content flags off and showLeaderLines=1; with no visible labels that flag is inert and must export. surface3D legend uses value
bands from the major-unit intervals (official gold: 0–20 / 20–40 /
40–60 / 60–80), not series names.

Residuals stay on the review page. Do not auto-mark pass.

## Still NI

- bar3DChart barDir=bar
- stacked / percentStacked 3D bar, line, or area
- non-box 3D shapes (cone, cylinder, pyramid, …)
- surface3DChart wireframe=1
- visible value labels on a 3D chart
- 3D plus secondary valAx
- Features Office itself cannot Chart.Export were skipped
