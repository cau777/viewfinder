import base64
import io
import json
from urllib.error import HTTPError

import pytest
from PIL import Image
from fastapi import HTTPException
from fastapi.testclient import TestClient

from app import description, main
from app.view import NoDataError


@pytest.fixture
def png():
    image = Image.new("RGB", (8, 2))
    for x in range(8):
        for y in range(2):
            image.putpixel((x, y), (x, 0, 0))
    output = io.BytesIO()
    image.save(output, format="PNG")
    return output.getvalue()


def test_gemini_image_request_and_usage(monkeypatch, png):
    monkeypatch.setenv("GEMINI_API_KEY", "test-key")
    monkeypatch.delenv("GEMINI_MODEL", raising=False)

    def upstream(request, timeout):
        assert request.full_url.endswith("/gemini-3.5-flash-lite:generateContent")
        assert request.get_header("X-goog-api-key") == "test-key"
        assert timeout == 30
        payload = json.loads(request.data)
        parts = payload["contents"][0]["parts"]
        for i, (direction, expected) in enumerate([("North", [7, 0]), ("East", [1, 2]), ("South", [3, 4]), ("West", [5, 6])]):
            assert parts[1 + 2 * i]["text"].startswith(direction)
            image = parts[2 + 2 * i]["inlineData"]
            assert image["mimeType"] == "image/png"
            crop = Image.open(io.BytesIO(base64.b64decode(image["data"])))
            assert crop.size == (2, 2)
            assert [crop.getpixel((x, 0))[0] for x in range(2)] == expected
        return io.BytesIO(json.dumps({
            "candidates": [{"finishReason": "STOP", "content": {"parts": [
                {"text": "Internal thought", "thought": True},
                {"text": "Trees surround an open view."},
            ]}}],
            "usageMetadata": {"promptTokenCount": 1200, "candidatesTokenCount": 20, "thoughtsTokenCount": 5},
        }).encode())

    monkeypatch.setattr(description, "urlopen", upstream)
    result = description.describe_panorama(png)
    assert result.description == "Trees surround an open view."
    assert (result.input_tokens, result.output_tokens, result.thinking_tokens) == (1200, 20, 5)


@pytest.mark.parametrize("failure,status", [("missing-key", 503), ("timeout", 504), ("rate-limit", 503), ("empty", 502)])
def test_upstream_failures(monkeypatch, failure, status, png):
    monkeypatch.setenv("GEMINI_API_KEY", "test-key")
    if failure == "missing-key":
        monkeypatch.delenv("GEMINI_API_KEY")

    def upstream(*args, **kwargs):
        if failure == "timeout":
            raise TimeoutError()
        if failure == "rate-limit":
            raise HTTPError("https://example.test", 429, "secret upstream body", {}, None)
        return io.BytesIO(b'{"candidates": []}')

    monkeypatch.setattr(description, "urlopen", upstream)
    with pytest.raises(HTTPException) as exc:
        description.describe_panorama(png)
    assert exc.value.status_code == status
    assert "secret" not in exc.value.detail


def test_separate_endpoint_and_image_independence(monkeypatch):
    main.app.dependency_overrides[main.get_tracer] = lambda: object()
    monkeypatch.setattr(main, "panorama_png", lambda *args: b"test-png")

    def unavailable(png):
        raise HTTPException(status_code=504, detail="AI timed out")

    monkeypatch.setattr(main, "describe_panorama", unavailable)
    try:
        client = TestClient(main.app)
        point = {"latitude": 49.28, "longitude": -123.12}
        assert client.post("/api/panorama/description", json=point).status_code == 504
        assert client.get("/api/panorama.png", params=point).content == b"test-png"
        assert client.post("/api/panorama/description", json={**point, "latitude": 100}).status_code == 422

        monkeypatch.setattr(main, "describe_panorama", lambda png: description.DescriptionResponse(description="Open view.", model="test"))
        assert client.post("/api/panorama/description", json=point).json()["description"] == "Open view."

        def no_data(*args):
            raise NoDataError("No LiDAR data at this position")

        monkeypatch.setattr(main, "panorama_png", no_data)
        assert client.post("/api/panorama/description", json=point).status_code == 404
    finally:
        main.app.dependency_overrides.clear()
