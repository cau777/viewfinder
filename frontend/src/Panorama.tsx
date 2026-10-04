import { useEffect, useRef, useState, type PointerEvent, type KeyboardEvent, type ReactNode } from 'react'
import { panoramaUrl, type Coordinates } from './api'

const COMPASS = [[0, 'N'], [45, 'NE'], [90, 'E'], [135, 'SE'], [180, 'S'], [225, 'SW'], [270, 'W'], [315, 'NW']] as const
const KEY_STEP = 10 // degrees per arrow key press

function compassName(bearing: number) {
  return COMPASS[Math.round(bearing / 45) % 8][1]
}

type Props = {
  point: Coordinates
  /** Bearing (degrees clockwise from north) to centre on first, e.g. the longest sightline */
  initialBearing: number | null
  /** Shown while the image loads, and if it fails (no data at the point, server error) */
  fallback: ReactNode
  onLoad: (point: Coordinates) => void
}

/** The 360° panorama around a point, dragged sideways to look around. It wraps at north: the image
 * repeats, and the offset is kept within one turn. */
export default function Panorama({ point, initialBearing, fallback, onLoad }: Props) {
  const container = useRef<HTMLDivElement>(null)
  const drag = useRef<{ x: number; offset: number } | null>(null)
  const centred = useRef(false)
  const url = panoramaUrl(point)
  // The last image loaded; it belongs to an earlier point while the new one loads
  const [loaded, setLoaded] = useState<{ url: string; aspect: number } | null>(null)
  const image = loaded?.url === url ? loaded : null
  const [height, setHeight] = useState(0)
  const [containerWidth, setContainerWidth] = useState(0)
  /** Bearing at the centre of the panel */
  const [bearing, setBearing] = useState(0)

  useEffect(() => {
    let cancelled = false
    const preload = new Image()
    preload.onload = () => {
      if (!cancelled) {
        setLoaded({ url, aspect: preload.naturalWidth / preload.naturalHeight })
        onLoad(point)
      }
    }
    preload.src = url
    centred.current = false
    return () => { cancelled = true; preload.onload = null }
  }, [url, point, onLoad])

  useEffect(() => {
    const element = container.current
    if (!element) return
    const observer = new ResizeObserver(() => { setHeight(element.clientHeight); setContainerWidth(element.clientWidth) })
    observer.observe(element)
    return () => observer.disconnect()
  }, [image])

  // Centre on the requested bearing once per point, unless the user already looked around
  useEffect(() => {
    if (image && initialBearing !== null && !centred.current) {
      setBearing(initialBearing)
      centred.current = true
    }
  }, [image, initialBearing])

  if (!image) return <>{fallback}</>

  const width = height * image.aspect // pixels for 360°
  const toPixels = (degrees: number) => degrees / 360 * width
  // Track is two turns wide; shift it so `bearing` lands in the middle of the panel
  const offset = width ? ((toPixels(bearing) - containerWidth / 2) % width + width) % width : 0
  const turn = (degrees: number) => setBearing(value => ((value + degrees) % 360 + 360) % 360)

  function onPointerDown(event: PointerEvent<HTMLDivElement>) {
    event.currentTarget.setPointerCapture(event.pointerId)
    drag.current = { x: event.clientX, offset: bearing }
    centred.current = true
  }
  function onPointerMove(event: PointerEvent<HTMLDivElement>) {
    if (!drag.current || !width) return
    // Dragging right brings what is to the left (counter-clockwise) into view
    const degrees = drag.current.offset - (event.clientX - drag.current.x) / width * 360
    setBearing((degrees % 360 + 360) % 360)
  }
  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      event.preventDefault()
      centred.current = true
      turn(event.key === 'ArrowLeft' ? -KEY_STEP : KEY_STEP)
    }
  }

  return (
    <div ref={container} className="panorama" role="slider" tabIndex={0} aria-label="360° view from this location. Drag or use the arrow keys to look around."
      aria-valuemin={0} aria-valuemax={359} aria-valuenow={Math.round(bearing) % 360} aria-valuetext={`Facing ${compassName(bearing)}, ${Math.round(bearing) % 360}°`}
      onPointerDown={onPointerDown} onPointerMove={onPointerMove} onPointerUp={() => { drag.current = null }} onPointerCancel={() => { drag.current = null }} onKeyDown={onKeyDown}>
      <div className="panorama-track" style={{ width: 2 * width, transform: `translateX(${-offset}px)`, backgroundImage: `url(${image.url})`, backgroundSize: `${width}px 100%` }}>
        {[0, 360].flatMap(turnStart => COMPASS.map(([degrees, name]) =>
          <span key={turnStart + degrees} className={`compass-mark${name.length === 1 ? ' cardinal' : ''}`} style={{ left: toPixels(turnStart + degrees) }}>{name}</span>))}
      </div>
      <span className="panorama-heading">Facing {compassName(bearing)} · {Math.round(bearing) % 360}°</span>
    </div>
  )
}
