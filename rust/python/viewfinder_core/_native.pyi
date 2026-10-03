from typing import Final, final
__version__: Final[str]

@final
class ByteBuffer:
    """
    A Python module implemented in Rust.
    """
    def __new__(cls, /, capacity: int = 1024) -> ByteBuffer: ...
    def push(self, /, chunk: "str | bytes | bytearray | list[int]") -> int: ...
