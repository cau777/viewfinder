import os
import threading
from pathlib import Path
from typing import Annotated

from fastapi import Depends, FastAPI, HTTPException
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles
from pydantic import BaseModel, Field

from viewfinder_core import RayTracer
from app.view import NoDataError, NoObstructionError, compute_view

from app.analysis import analyze

app = FastAPI(title="viewfinder", version="0.1.0")

# The u16 LiDAR export (index.csv + <NAME>.u16 files)
DATA_DIR = Path(os.environ.get("VIEWFINDER_DATA_DIR", "~/data/lidar-vancouver-u16")).expanduser()

_tracer: RayTracer | None = None
_tracer_lock = threading.Lock()


def get_tracer() -> RayTracer:
    """The grid, loaded on first use (a few seconds) and shared by every request of this process."""
    global _tracer
    with _tracer_lock:
        if _tracer is None:
            if not (DATA_DIR / "index.csv").is_file():
                raise HTTPException(status_code=503, detail=f"LiDAR data not found in {DATA_DIR}")
            _tracer = RayTracer(DATA_DIR)
        return _tracer


class ViewRequest(BaseModel):
    latitude: float = Field(ge=-90, le=90)
    longitude: float = Field(ge=-180, le=180)
    bearings: int = Field(default=360*4, ge=4, le=3600, description="Number of rays around the observer")


class ViewPoint(BaseModel):
    bearing: float
    latitude: float
    longitude: float
    distance: float
    unobstructed: bool


class ViewResponse(BaseModel):
    latitude: float
    longitude: float
    ground_altitude: float
    altitude: float
    average_distance: float
    farthest_distance: float
    unobstructed_share: float
    points: list[ViewPoint]


@app.post("/api/view", response_model=ViewResponse)
def view(req: ViewRequest, tracer: Annotated[RayTracer, Depends(get_tracer)]) -> ViewResponse:
    """The area visible from a position: where horizontal rays in every direction hit the surface.
    Rays that hit nothing are limited to the average sightline."""
    try:
        result = compute_view(tracer, req.latitude, req.longitude, req.bearings)
    except NoDataError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc
    except NoObstructionError as exc:
        raise HTTPException(status_code=422, detail=str(exc)) from exc
    return ViewResponse.model_validate(result, from_attributes=True)


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
