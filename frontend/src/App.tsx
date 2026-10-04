import { useEffect, useRef, useState } from 'react'
import L from 'leaflet'
import AddressSearch from './AddressSearch'
import Panorama from './Panorama'
import PanoramaDescription from './PanoramaDescription'
import 'leaflet/dist/leaflet.css'
import { fetchView, type Coordinates, type View } from './api'

const VANCOUVER: L.LatLngExpression = [49.2827, -123.1207]
const pinIcon = L.divIcon({ className: 'viewfinder-pin', html: '<span></span>', iconSize: [32, 40], iconAnchor: [16, 40] })
// Area covered by the LiDAR export (outer corners of its tiles, from index.csv): the map can't leave it
const LIDAR_BOUNDS = L.latLngBounds([49.1937, -123.2750], [49.3197, -123.0137])
// Keep the geometry stable across zoom levels; Canvas clips drawing to its viewport.
const VIEW_STYLE: L.PolylineOptions = { color: '#FF6B6B', weight: 2, fillColor: '#FF6B6B', fillOpacity: 0.18, smoothFactor: 0, noClip: true, interactive: false }
const UNOBSTRUCTED_STYLE: L.CircleMarkerOptions = { radius: 2.5, color: '#FFD166', weight: 0, fillOpacity: 1, interactive: false }

/** Where a ray from `origin` towards `bearing` (degrees clockwise from north) leaves the LiDAR bounds.
 * Latitude and longitude are treated as a flat grid scaled by cos(latitude), accurate over a city. */
function toBoundsEdge(origin: Coordinates, bearing: number): L.LatLngTuple {
  const radians = bearing * Math.PI / 180
  const step = { latitude: Math.cos(radians), longitude: Math.sin(radians) / Math.cos(origin.latitude * Math.PI / 180) }
  const toEdge = (from: number, delta: number, low: number, high: number) =>
    delta > 0 ? (high - from) / delta : delta < 0 ? (low - from) / delta : Infinity
  const t = Math.max(0, Math.min(
    toEdge(origin.latitude, step.latitude, LIDAR_BOUNDS.getSouth(), LIDAR_BOUNDS.getNorth()),
    toEdge(origin.longitude, step.longitude, LIDAR_BOUNDS.getWest(), LIDAR_BOUNDS.getEast()),
  ))
  return [origin.latitude + step.latitude * t, origin.longitude + step.longitude * t]
}

function formatDistance(metres: number) {
  return metres < 1000 ? `${Math.round(metres)} m` : `${(metres / 1000).toFixed(1)} km`
}

export default function App() {
  const container = useRef<HTMLDivElement>(null)
  const map = useRef<L.Map | null>(null)
  const marker = useRef<L.Marker | null>(null)
  const viewLayer = useRef<L.LayerGroup | null>(null)
  const [satellite, setSatellite] = useState(false)
  const [point, setPoint] = useState<Coordinates | null>(null)
  const [view, setView] = useState<View | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [mapError, setMapError] = useState(false)
  const [attempt, setAttempt] = useState(0)
  const [panoramaReady, setPanoramaReady] = useState<Coordinates | null>(null)

  useEffect(() => {
    if (!container.current) return
    const instance = L.map(container.current, { zoomControl: false, renderer: L.canvas(), maxBounds: LIDAR_BOUNDS, maxBoundsViscosity: 1 }).setView(VANCOUVER, 13)
    // Keep the entire viewport inside coverage, even on wide screens or after resizing.
    // `inside: true` finds the zoom at which the viewport fits within the data bounds.
    const fitMinZoom = () => {
      instance.setMinZoom(instance.getBoundsZoom(LIDAR_BOUNDS, true))
      instance.panInsideBounds(LIDAR_BOUNDS, { animate: false })
    }
    fitMinZoom()
    instance.on('resize', fitMinZoom)
    map.current = instance
    viewLayer.current = L.layerGroup().addTo(instance)
    instance.on('click', ({ latlng }: L.LeafletMouseEvent) => {
      marker.current?.remove()
      marker.current = L.marker(latlng, { icon: pinIcon }).addTo(instance)
      setPoint({ latitude: latlng.lat, longitude: latlng.lng })
    })
    return () => { instance.remove(); map.current = null; marker.current = null; viewLayer.current = null }
  }, [])

  useEffect(() => {
    const instance = map.current
    if (!instance) return
    setMapError(false)
    const tiles = satellite
      ? L.tileLayer('https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}', {
          attribution: 'Tiles &copy; <a href="https://www.esri.com/">Esri</a> — Esri, Maxar, Earthstar Geographics, and the GIS User Community',
          maxNativeZoom: 19,
          maxZoom: 20,
        })
      : L.tileLayer(`https://basemaps.cartocdn.com/rastertiles/voyager/{z}/{x}/{y}{r}.png?key=${encodeURIComponent(__CARTO_API_KEY__)}`, {
          className: 'coastal-basemap',
          attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors &copy; <a href="https://carto.com/attributions">CARTO</a>',
          maxZoom: 20,
        })
    tiles.on('tileerror', () => setMapError(true))
    tiles.on('tileload', () => setMapError(false))
    tiles.addTo(instance)
    return () => { tiles.off(); tiles.remove() }
  }, [satellite])

  useEffect(() => {
    if (!point) return
    const controller = new AbortController()
    setLoading(true)
    setView(null)
    setError(null)
    fetchView(point, controller.signal)
      .then(setView)
      .catch((reason: unknown) => { if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : 'Unable to load this view.') })
      .finally(() => { if (!controller.signal.aborted) setLoading(false) })
    return () => controller.abort()
  }, [point, attempt])

  // The visible area: one vertex per ray, where it hits the surface or, if nothing is in the way, at the edge of the LiDAR area
  useEffect(() => {
    const layer = viewLayer.current
    if (!layer) return
    layer.clearLayers()
    if (!view) return
    const vertices = view.points.map(p => p.unobstructed ? toBoundsEdge(view, p.bearing) : [p.latitude, p.longitude] as L.LatLngTuple)
    L.polygon(vertices, VIEW_STYLE).addTo(layer)
    let lastMarkerBearing = -Infinity
    view.points.forEach((p, i) => {
      if (p.unobstructed && p.bearing - lastMarkerBearing >= 4) {
        L.circleMarker(vertices[i], UNOBSTRUCTED_STYLE).addTo(layer)
        lastMarkerBearing = p.bearing
      }
    })
  }, [view])

  const hits = view?.points.filter(p => !p.unobstructed) ?? []
  const unobstructedShare = view ? view.unobstructed_share ?? (view.points.length ? 1 - hits.length / view.points.length : 0) : 0
  const farthestDistance = view?.farthest_distance ?? hits.reduce((maximum, p) => Math.max(maximum, p.distance), 0)
  // The panorama opens facing the longest sightline
  const farthestBearing = hits.length ? hits.reduce((farthest, p) => p.distance > farthest.distance ? p : farthest).bearing : null

  function closePanel() {
    setPoint(null)
    setView(null)
    marker.current?.remove()
    marker.current = null
  }

  return (
    <div className="app-shell">
      <header className="navbar">
        <a className="brand" href="/" aria-label="ViewFinder home"><span className="brand-icon">⌖</span>ViewFinder<span className="brand-dot">.</span></a>
        <AddressSearch onSelect={location => {
          if (!map.current) return
          const latlng = L.latLng(location.latitude, location.longitude)
          marker.current?.remove()
          marker.current = L.marker(latlng, { icon: pinIcon }).addTo(map.current)
          map.current.setView(latlng, 16)
          setPoint({ latitude: location.latitude, longitude: location.longitude })
        }} />
      </header>
      <main className="map-shell">
        <div ref={container} className="map" aria-label="Interactive map of Vancouver. Click a location to discover its view." />
        {mapError && <div className="map-error" role="alert">{satellite ? 'Satellite imagery could not load. Check your connection or switch to the street map.' : 'Map tiles could not load. Check your connection and CARTO basemap key.'}</div>}
        <div className="basemap-toggle" role="group" aria-label="Map style">
          <button type="button" aria-pressed={!satellite} onClick={() => setSatellite(false)}>Map</button>
          <button type="button" aria-pressed={satellite} onClick={() => setSatellite(true)}>Satellite</button>
        </div>
        <div className="map-controls">
          <button onClick={() => map.current?.zoomIn()} aria-label="Zoom in">+</button>
          <button onClick={() => map.current?.zoomOut()} aria-label="Zoom out">−</button>
          <button className="recenter" onClick={() => map.current?.setView(VANCOUVER, 13)} aria-label="Return to Vancouver" title="Return to Vancouver">⌖</button>
        </div>
        {!point && <div className="map-hint"><span className="hint-pin">⌖</span><div><span>Click anywhere on the map to analyze the gorgeous view at that location</span></div></div>}
        {point && <aside className="view-panel" aria-label="Selected location" aria-busy={loading}>
          <div className="panel-top"><span className="eyebrow">YOUR PERSPECTIVE</span><button className="close-button" onClick={closePanel} aria-label="Close location details">×</button></div>
          <div className="view-art"><Panorama point={point} initialBearing={farthestBearing} onLoad={setPanoramaReady} fallback={<div aria-hidden="true"><div className="sun" /><div className="mountain mountain-back" /><div className="mountain mountain-front" /><div className="water" /><span className="art-label">A PLACE TO PAUSE</span></div>} /></div>
          <p className="panorama-hint">Drag to rotate your perspective.</p>
          <div className="panel-content"><div className="location-tag"><span className="status-dot" /> PINNED LOCATION</div><h2>{loading ? 'Finding your view…' : view ? `You can see ${formatDistance(view.average_distance)} around` : 'Your selected view'}</h2><p className="coordinates">{point.latitude.toFixed(5)}, {point.longitude.toFixed(5)}</p>
            {panoramaReady === point && <PanoramaDescription key={`${point.latitude},${point.longitude}`} point={point} />}
            <div aria-live="polite">{loading ? <div className="loading-state"><span className="spinner" /><p>Taking a look around.<br /><span>Your perspective is on its way.</span></p></div> : error ? <div role="alert"><p>{error}</p><button className="retry-button" onClick={() => setAttempt(value => value + 1)}>Try again ↗</button></div> : view && <><div className="description-heading"><span>↗</span><h3>From where you stand</h3></div><p className="description">The shaded area on the map is what you can see at eye level, {formatDistance(view.altitude - view.ground_altitude)} above the ground, before buildings, trees or terrain block the view.</p><dl className="view-stats"><div><dt>Eye altitude</dt><dd>{Math.round(view.altitude)} m</dd></div><div><dt>Average sightline</dt><dd>{formatDistance(view.average_distance)}</dd></div><div><dt>Farthest sightline</dt><dd>{formatDistance(farthestDistance)}</dd></div><div><dt>Open directions</dt><dd>{Math.round(unobstructedShare * 100)}%</dd></div></dl>{unobstructedShare > 0 && <p className="view-legend"><span className="legend-dot" />Nothing blocks the view in these directions; they extend to the edge of the LiDAR coverage.</p>}</>}</div>
          </div><div className="panel-footer">A different view is just a click away.<span>⌖</span></div>
        </aside>}
      </main>
    </div>
  )
}
