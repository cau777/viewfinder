from fastapi.testclient import TestClient

from app.main import app
from viewfinder_core import ByteBuffer

client = TestClient(app)


def test_rust_buffer_directly():
    buf = ByteBuffer(8)
    assert buf.push("hi") == 2
    assert buf.push(b"\x00\x01") == 4
    assert buf.push([255]) == 5
    assert len(buf) == 5 and buf.capacity == 8
    assert buf.to_bytes() == b"hi\x00\x01\xff"
    s = buf.summary(preview=2)
    assert (s.length, s.capacity, s.hex_preview) == (5, 8, "68 69")
    assert s.histogram_top[0] == (0, 1)  # ties broken by byte value


def test_analyze_end_to_end():
    r = client.post("/api/analyze", json={"text": "Wikipedia"})
    assert r.status_code == 200, r.text
    body = r.json()
    assert body["length"] == 9
    assert body["checksum"] == 0x11E60398
    assert body["histogram_top"][0] == {"byte": ord("i"), "char": "i", "count": 3}


def test_overflow_maps_to_422():
    r = client.post("/api/analyze", json={"text": "abcd", "repeat": 3, "capacity": 10})
    assert r.status_code == 422
    assert "exceeds capacity" in r.json()["detail"]
