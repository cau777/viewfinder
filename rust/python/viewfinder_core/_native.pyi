"""
Native core of viewfinder (Rust, via PyO3).
"""

from typing import Final, final
__version__: Final[str]

@final
class ByteBuffer:
    """
    A fixed-capacity byte buffer owned by Rust.
    
    Feed it Python objects with `push()`; inspect it with `summary()`.
    """
    def __len__(self, /) -> int: ...
    def __new__(cls, /, capacity: int = 1024) -> ByteBuffer:
        """
        ByteBuffer(capacity=1024)
        """
    def __repr__(self, /) -> str: ...
    @property
    def capacity(self, /) -> int: ...
    def clear(self, /) -> None: ...
    def push(self, /, chunk: "str | bytes | bytearray | list[int]") -> int:
        """
        Append a Python object to the buffer and return the new length.
        
        Accepts `str` (UTF-8 encoded), `bytes`, `bytearray`, or any iterable
        of ints in 0..=255 (e.g. `list[int]`, `memoryview`).
        """
    def summary(self, /, preview: int = 16) -> Summary:
        """
        Describe the buffer contents. `preview` caps how many bytes appear in `hex_preview`.
        """
    def to_bytes(self, /) -> bytes:
        """
        Copy the buffer out as Python `bytes`.
        """

@final
class Summary:
    """
    Snapshot of a ByteBuffer, returned by `ByteBuffer.summary()`.
    """
    def __repr__(self, /) -> str: ...
    @property
    def capacity(self, /) -> int:
        """
        Maximum number of bytes the buffer accepts.
        """
    @property
    def checksum(self, /) -> int:
        """
        Adler-32 checksum of the contents.
        """
    @property
    def hex_preview(self, /) -> str:
        """
        Space-separated hex of the first `preview` bytes.
        """
    @property
    def histogram_top(self, /) -> list[tuple[int, int]]:
        """
        Up to 5 most frequent `(byte, count)` pairs, most frequent first.
        """
    @property
    def length(self, /) -> int:
        """
        Bytes currently stored.
        """
