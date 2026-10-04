"""Non-streaming Gemini descriptions, independent of the panorama image response.

Set GEMINI_API_KEY in the server environment. GEMINI_MODEL optionally overrides
the default Flash-Lite model. Pillow splits the panorama into directional crops.
"""

import base64
import json
import io
import os
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

from fastapi import HTTPException
from pydantic import BaseModel, Field
from PIL import Image


class DescriptionRequest(BaseModel):
    latitude: float = Field(ge=-90, le=90)
    longitude: float = Field(ge=-180, le=180)


class DescriptionResponse(BaseModel):
    description: str
    model: str
    input_tokens: int = 0
    output_tokens: int = 0
    thinking_tokens: int = 0


PROMPT = """Describe the view around an observer in 2–3 concise sentences (at most 80 words).
These four labeled images are adjacent 90-degree crops of a synthetic LiDAR
panorama, not photographs. Each crop is centered on its labeled compass
direction; within each crop, bearings increase clockwise. The vertical range is
20 degrees above to 10 degrees below the horizon. The observer is 2.5 m above
the surface. Green represents vegetation (darker green is taller vegetation),
gray buildings or bridge decks, tan ground, dark gray roads or rail, and blue
water. Distant surfaces fade toward pale blue-gray. Rays that leave the dataset
without a collision use synthetic sky colors, even below the horizon; those
colors do not establish that water is present. Do not infer weather, time of
day, or actual sky conditions.
Focus on the most prominent obstructions and whether there is a broad distant
view. Full-height solid color regions are nearby surfaces blocking the view;
do not describe them as open ground. Green areas are vegetation, not rolling
hills. Do not claim water unless a distinct blue surface is visible at the
bottom of a crop. Do not describe actual sky. Avoid claims about foreground
ground, roads, or landforms when ambiguous. Only mention compass directions
supported by the labeled crops. Do not invent landmarks, building
names, people, exact distances, or details that the rendering cannot establish.
Use plain, welcoming language, with no headings or discussion of the rendering.
"""


def panorama_parts(png: bytes) -> list[dict]:
    """Quarter-turn crops give the thin 360° strip more usable vision detail."""
    with Image.open(io.BytesIO(png)) as source:
        image = source.convert("RGB")
    width, height = image.size
    wrapped = Image.new("RGB", (width + width // 4, height))
    wrapped.paste(image, (0, 0))
    wrapped.paste(image.crop((0, 0, width // 4, height)), (width, 0))
    parts = [{"text": PROMPT}]
    for direction, start in [("North", 7 * width // 8), ("East", width // 8),
                             ("South", 3 * width // 8), ("West", 5 * width // 8)]:
        crop = wrapped.crop((start, 0, start + width // 4, height))
        output = io.BytesIO()
        crop.save(output, format="PNG")
        parts.extend([
            {"text": f"{direction}-facing sector (90 degrees):"},
            {"inlineData": {"mimeType": "image/png", "data": base64.b64encode(output.getvalue()).decode("ascii")}},
        ])
    return parts


def describe_panorama(png: bytes) -> DescriptionResponse:
    """One bounded request; called from a sync FastAPI route's worker thread."""
    api_key = os.environ.get("GEMINI_API_KEY")
    if not api_key:
        raise HTTPException(status_code=503, detail="Panorama descriptions require GEMINI_API_KEY")
    model = os.environ.get("GEMINI_MODEL", "gemini-3.5-flash-lite")
    payload = {
        "contents": [{"role": "user", "parts": panorama_parts(png)}],
        "generationConfig": {
            "maxOutputTokens": 512,
            "thinkingConfig": {"thinkingLevel": "MINIMAL"},
        },
    }
    request = Request(
        f"https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent",
        data=json.dumps(payload).encode(),
        headers={"Content-Type": "application/json", "x-goog-api-key": api_key},
        method="POST",
    )
    try:
        with urlopen(request, timeout=30) as response:
            result = json.load(response)
    except HTTPError as exc:
        # Never expose upstream response bodies or the API key to the client.
        status = 503 if exc.code == 429 else 502
        raise HTTPException(status_code=status, detail="Panorama description service unavailable") from exc
    except (TimeoutError, URLError) as exc:
        raise HTTPException(status_code=504, detail="Panorama description service did not respond") from exc
    except (ValueError, UnicodeDecodeError) as exc:
        raise HTTPException(status_code=502, detail="Invalid panorama description response") from exc

    try:
        candidate = result.get("candidates", [{}])[0]
        parts = candidate.get("content", {}).get("parts", [])
        description = "".join(part.get("text", "") for part in parts if not part.get("thought")).strip()
        if not description or candidate.get("finishReason") != "STOP":
            raise ValueError("No complete description")
        usage = result.get("usageMetadata", {})
        return DescriptionResponse(
            description=description, model=model,
            input_tokens=usage.get("promptTokenCount", 0),
            output_tokens=usage.get("candidatesTokenCount", 0),
            thinking_tokens=usage.get("thoughtsTokenCount", 0),
        )
    except (AttributeError, IndexError, TypeError, ValueError) as exc:
        raise HTTPException(status_code=502, detail="No complete panorama description returned") from exc
