import { useEffect, useRef, useState } from 'react'
import L from 'leaflet'
import AddressSearch from './AddressSearch'
import 'leaflet/dist/leaflet.css'
import { describeView, type Coordinates, type ViewDescription } from './api'

const VANCOUVER: L.LatLngExpression = [49.2827, -123.1207]
const pinIcon = L.divIcon({ className: 'viewfinder-pin', html: '<span></span>', iconSize: [32, 40], iconAnchor: [16, 40] })

export default function App() {
  const container = useRef<HTMLDivElement>(null)
  const map = useRef<L.Map | null>(null)
  const marker = useRef<L.Marker | null>(null)
  const [satellite, setSatellite] = useState(false)
  const [point, setPoint] = useState<Coordinates | null>(null)
  const [view, setView] = useState<ViewDescription | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [mapError, setMapError] = useState(false)
  const [attempt, setAttempt] = useState(0)

  useEffect(() => {
    if (!container.current) return
    const instance = L.map(container.current, { zoomControl: false }).setView(VANCOUVER, 13)
    map.current = instance
    instance.on('click', ({ latlng }: L.LeafletMouseEvent) => {
      marker.current?.remove()
      marker.current = L.marker(latlng, { icon: pinIcon }).addTo(instance)
      setPoint({ latitude: latlng.lat, longitude: latlng.lng })
    })
    return () => { instance.remove(); map.current = null; marker.current = null }
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
      : L.tileLayer(`https://basemaps.cartocdn.com/rastertiles/dark_all/{z}/{x}/{y}{r}.png?key=${encodeURIComponent(__CARTO_API_KEY__)}`, {
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
    describeView(point, controller.signal)
      .then(setView)
      .catch((reason: unknown) => { if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : 'Unable to load this view.') })
      .finally(() => { if (!controller.signal.aborted) setLoading(false) })
    return () => controller.abort()
  }, [point, attempt])

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
        <section className="map-intro"><span className="eyebrow">EXPLORE YOUR PERSPECTIVE</span><h1>Every place has a view.</h1><p>Drop a pin. Discover what’s around you.</p></section>
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
        {!point && <div className="map-hint"><span className="hint-pin">⌖</span><div><strong>Start with a little curiosity</strong><span>Click anywhere on the map to find your view</span></div><span className="hint-arrow">↗</span></div>}
        {point && <aside className="view-panel" aria-label="Selected location" aria-busy={loading}>
          <div className="panel-top"><span className="eyebrow">YOUR PERSPECTIVE</span><button className="close-button" onClick={closePanel} aria-label="Close location details">×</button></div>
          <div className="view-art" aria-hidden="true"><div className="sun" /><div className="mountain mountain-back" /><div className="mountain mountain-front" /><div className="water" /><span className="art-label">A PLACE TO PAUSE</span></div>
          <div className="panel-content"><div className="location-tag"><span className="status-dot" /> PINNED LOCATION</div><h2>{loading ? 'Finding your view…' : view?.title ?? 'Your selected view'}</h2><p className="coordinates">{point.latitude.toFixed(5)}, {point.longitude.toFixed(5)}</p>
            <div aria-live="polite">{loading ? <div className="loading-state"><span className="spinner" /><p>Taking a look around.<br /><span>Your perspective is on its way.</span></p></div> : error ? <div role="alert"><p>{error}</p><button className="retry-button" onClick={() => setAttempt(value => value + 1)}>Try again ↗</button></div> : view && <><div className="description-heading"><span>↗</span><h3>From where you stand</h3></div><p className="description">{view.description}</p><div className="view-tags">{view.tags.map(tag => <span key={tag}>{tag}</span>)}</div><div className="mock-note"><span>✧</span> A preview of what’s possible<p>This is an illustrative description. Live location insights are coming soon.</p></div></>}</div>
          </div><div className="panel-footer">A different view is just a click away.<span>⌖</span></div>
        </aside>}
      </main>
    </div>
  )
}
