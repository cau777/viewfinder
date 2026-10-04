# Panorama description prototype

The image remains at `GET /api/panorama.png?latitude=...&longitude=...`.
Descriptions are separate, non-streaming requests:

```http
POST /api/panorama/description
Content-Type: application/json

{"latitude": 49.2827, "longitude": -123.1207}
```

The JSON response contains `description`, `model`, `input_tokens`,
`output_tokens`, and `thinking_tokens`. Token counts are Gemini's reported usage.
The default model is `gemini-3.5-flash-lite`, with minimal thinking.
The server sends four labeled 90-degree crops (north, east, south, west) so
the model can read the narrow panoramic image and distinguish directions.

Supply `GEMINI_API_KEY` in the server environment. To load the repository's
existing `.env` file when starting the API from `backend/`:

```sh
uv run --env-file ../.env --no-sync uvicorn app.main:app --host 127.0.0.1 --port 8000
```

Optionally set `GEMINI_MODEL` to another model supporting minimal thinking.
The key stays on the server and is sent to Google in a request header.

The client should display the panorama immediately, then POST its coordinates
from the image's load callback. Show a separate description loading state.
Abort or ignore stale requests when the selected location changes; a description
failure should not remove the image. No polling or token streaming is needed.
Requests can also start in parallel, but waiting for the image to load prioritizes
the panorama and avoids AI requests for images that fail to load.

This server-only prototype regenerates the deterministic PNG for the description.
It makes one Gemini request with a 30-second network timeout and no automatic
retries. Missing credentials return 503; upstream failures return 502/503/504;
locations without LiDAR data return 404. Descriptions are not cached yet, so
repeated calls incur repeated API usage. Before public rollout, add a bounded
render/description cache and request limits. The client requests a description
after its panorama loads and displays loading, retry, and failure states separately.
When using `make dev-remote`, the Jupyter backend must also have this endpoint deployed.

## Live validation

Tested through the Jupyter-backed client on October 4, 2026. All three locations
returned HTTP 200, 4,689 input tokens, and zero thinking tokens. Descriptions were
checked visually against the PNGs; the prompt was tightened after an initial
round overstated open ground and misplaced directions.

| Location | Coordinates | Description emphasis | PNG latency | AI latency | Estimated USD |
| --- | --- | --- | --- | --- | --- |
| Downtown | 49.2827, -123.1207 | Nearby structures; vegetation toward the south | 0.05 s | 7.19 s | $0.00151 |
| Queen Elizabeth Park | 49.2414, -123.1126 | Prominent greenery with distant buildings | 0.06 s | 2.30 s | $0.00157 |
| Kitsilano waterfront | 49.2734, -123.1554 | Open water to the north; vegetation blocking the south | 0.04 s | 11.57 s | $0.00154 |

Costs use $0.30 per million input tokens and $2.50 per million output tokens.
Timings are individual observations, not latency guarantees. A real browser
check also confirmed that the colored panorama appeared while the description
was loading, and that the completed description appeared in Your surroundings.
