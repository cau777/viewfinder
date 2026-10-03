"""Plain-Python layer between the HTTP endpoint and the Rust core."""

from viewfinder_core import ByteBuffer


def analyze_text(text: str, repeat: int = 1, capacity: int = 4096) -> dict:
    """Load `text` into a Rust-owned buffer `repeat` times and summarise it.

    Raises ValueError if the encoded payload would exceed `capacity`
    (the error originates in Rust and surfaces as a Python ValueError).
    """
    buf = ByteBuffer(capacity)
    for _ in range(repeat):
        buf.push(text)  # str -> UTF-8 bytes, converted inside Rust
    s = buf.summary(preview=24)  # typed viewfinder_core.Summary (see _native.pyi)
    return {
        "length": s.length,
        "capacity": s.capacity,
        "checksum": s.checksum,
        "hex_preview": s.hex_preview,
        "histogram_top": [
            {"byte": b, "char": chr(b) if 32 <= b < 127 else None, "count": c}
            for b, c in s.histogram_top
        ],
    }
