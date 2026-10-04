# View data inventory

Implementation update: the selected option 3 dashboard is now in the app. POST /api/view additionally returns nullable `analysis` containing `date`, `beauty_score`, `sunrise`/`sunset` (`time`, `bearing` in degrees, `open_share` from 0 to 1), `ocean_area`, `lake_area`, `water_area`, `openness_area` in m², and `landmarks`. The sidebar shows score, sun information, ocean overlap and landmarks, with elevation and method collapsed. Average sightline remains in the API but is no longer displayed. Missing geographic layers yield null analysis while preserving core view data. Analysis dates now refresh per request and sun percentages are normalized before scoring. The inventory below records the initial audit before this implementation.

Audited on main after commit 68bbbd7. Companion: sidebar-options.html. Mockup values are invented examples, not measurements of a real location.

## Returned by POST /api/view

| Field | Meaning | Already used by client |
|---|---|---|
| latitude, longitude | Observer coordinates, degrees | Map geometry; selected request coordinates shown in sidebar |
| ground_altitude | Surface altitude, metres | Subtracted from eye altitude to explain observer height; not shown separately |
| altitude | Eye altitude, metres (surface + 2.5 m) | Sidebar eye altitude |
| average_distance | Mean distance of horizontal rays that hit a surface, metres; excludes open rays | Sidebar headline and average sightline |
| farthest_distance | Maximum collision distance across all original rays, metres | Sidebar farthest sightline; optional in frontend type with fallback |
| unobstructed_share | Fraction 0–1 of all original rays leaving the dataset without collision | Sidebar open directions percentage and legend; optional in frontend type with fallback |
| points[] | Thinned horizontal ray endpoints, preserving transitions and angular coverage | Map visible-area polygon and open-direction markers |
| points[].bearing | Degrees clockwise from north | Open-ray extension and markers; initial panorama heading chosen from longest returned blocked ray |
| points[].latitude, longitude | Endpoint coordinates | Blocked-ray polygon vertices |
| points[].distance | Collision distance, or average distance for open endpoints | Farthest-distance fallback; longest returned ray selection |
| points[].unobstructed | True when ray leaves dataset without collision | Open/blocked split, map extension, fallback coverage calculation |

Request accepts bearings (4–3600; default 1440). The client currently sends only coordinates. Bearings is not returned. Returned point count is not original ray count. Fallback estimates using thinned points may differ from the full-ray statistics. The farthest heading comes from thinned points, so it may not identify the actual maximum-distance ray.

Open directions mean no collision within LiDAR coverage, not confirmed infinite visibility. The backend caps open endpoints at mean collision distance; the map instead extends them to rectangular dataset bounds. Average distance is not the radius of the entire view. Horizontal view geometry is an eye-level approximation, not a full 3D visibility survey.

## Other responses and client state

| Information | Source | Client status |
|---|---|---|
| Synthetic 360° LiDAR panorama | GET /api/panorama.png | Shown; drag/arrow-key rotation; compass and current heading |
| Class and distance appearance | Embedded in PNG: buildings, vegetation, ground, water; distance fading | Visible as pixels, not exposed as numeric class totals or object records |
| Panorama dimensions | 1440 × 120; elevation −10° to +20°, north-first clockwise | Client reads image aspect ratio; not a metadata response |
| Surroundings description | POST /api/panorama/description: description | Requested after PNG loads; shown with independent loading/error/retry |
| model | Same description response | Returned over network, discarded by fetchDescription |
| input_tokens, output_tokens, thinking_tokens | Same description response | Returned over network, discarded; default zero |
| Address label and coordinates | Photon address search | Label appears in search results; selection passes coordinates only into view state, no persistent place title |
| Facing heading and compass label | Panorama component state | Shown; user-controlled, not an API metric |
| Eye height | altitude − ground_altitude | Used in explanatory text; formatter rounds 2.5 m to 3 m |
| Coverage bounds | App constant | Used for map constraints/open-ray extrapolation |
| Load/error/retry states | Client state/API errors | Present separately for view, description and map; panorama uses fallback art |

## Calculated internally, not returned or stored by the client

analyze() prints these values and returns None. Exposing them requires a structured analysis result, backend response fields, frontend types and UI wiring.

| Information | Current computation / unit |
|---|---|
| Sunrise / sunset direction | Lower, center, upper interval in radians (center ±0.1 radians) |
| Sunrise / sunset time | Datetime in America/Vancouver |
| Sunrise / sunset score | 0–100 percentage of rays with no collision in the sun-direction interval |
| Ocean intersection area | Polygon overlap with ocean layer, m² |
| Lake intersection area | Polygon overlap with lake layer, m² |
| Total water intersection area | Sum of ocean and lake overlap, m² |
| Landmarks | Names whose geometries intersect the polygon; not independently verified visible objects |
| Openness | Polygon area in projected metres, m² |
| Beauty score | Weighted composite, capped at 100 |
| Polygon | Shapely geometry held in backend View; omitted by response model |
| border_points / nonborder_points | Full-ray tuples (bearing, latitude, longitude), open/blocked; backend dataclass annotations currently say ViewPoint incorrectly |

Analysis caveats before exposing: date.today() is captured at module import, not refreshed each request; sun functions accept altitude but do not use it; sunrise/sunset scores return 0–100 but beauty calculation multiplies them by 10 as if they were 0–1, readily saturating the composite. Water and landmark results are polygon intersections, not independently validated line-of-sight findings. Analysis uses average-distance-capped open endpoints, unlike the client's extended map polygon. Openness area differs from unobstructed_share (angular coverage). Photos are loaded in features.py but no view endpoint returns photos or photo references.

## Source files

- backend/app/main.py — request/response contract and routes
- backend/app/view.py — geometry, metrics and panorama
- backend/app/analysis.py and features.py — internal analysis
- backend/app/description.py — description and usage response
- frontend/src/api.ts — client types and fetch helpers
- frontend/src/App.tsx — map and current sidebar
- frontend/src/Panorama.tsx and PanoramaDescription.tsx — image and description UI
