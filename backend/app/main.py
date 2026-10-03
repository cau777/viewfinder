import os
from pathlib import Path

from fastapi import FastAPI, HTTPException
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field

import viewfinder_core
from app.analysis import analyze_text

app = FastAPI(title="viewfinder", version="0.1.0")


class AnalyzeRequest(BaseModel):
    text: str = Field(max_length=10_000)
    repeat: int = Field(default=1, ge=1, le=100)
    capacity: int = Field(default=4096, ge=1, le=1_000_000)


class HistogramEntry(BaseModel):
    byte: int
    char: str | None
    count: int


class AnalyzeResponse(BaseModel):
    length: int
    capacity: int
    checksum: int
    hex_preview: str
    histogram_top: list[HistogramEntry]
    core_version: str


@app.post("/api/analyze", response_model=AnalyzeResponse)
def analyze(req: AnalyzeRequest) -> AnalyzeResponse:
    try:
        summary = analyze_text(req.text, req.repeat, req.capacity)
    except ValueError as exc:  # raised from Rust on capacity overflow
        raise HTTPException(status_code=422, detail=str(exc)) from exc
    return AnalyzeResponse(**summary, core_version=viewfinder_core.__version__)


# --- Production: serve the built React app from the same process ------------
# In development Vite serves the UI (with HMR) and proxies /api here instead.
STATIC_DIR = Path(os.environ.get("VIEWFINDER_STATIC_DIR", Path(__file__).resolve().parents[2] / "frontend" / "dist"))

if STATIC_DIR.is_dir():
    app.mount("/assets", StaticFiles(directory=STATIC_DIR / "assets"), name="assets")

    @app.get("/{full_path:path}", include_in_schema=False)
    def spa(full_path: str) -> FileResponse:
        if full_path.startswith("api/"):
            raise HTTPException(status_code=404)
        candidate = (STATIC_DIR / full_path).resolve()
        if full_path and candidate.is_file() and candidate.is_relative_to(STATIC_DIR.resolve()):
            return FileResponse(candidate)
        return FileResponse(STATIC_DIR / "index.html")  # client-side routing fallback