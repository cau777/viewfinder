import { useEffect, useId, useLayoutEffect, useRef, useState } from 'react'
import { fetchDescription, type Coordinates } from './api'

/** Reserve space immediately; request the description once the panorama is ready. */
export default function PanoramaDescription({ point, ready }: { point: Coordinates; ready: boolean }) {
  const [description, setDescription] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [attempt, setAttempt] = useState(0)
  const [expanded, setExpanded] = useState(false)
  const [overflows, setOverflows] = useState(false)
  const copy = useRef<HTMLParagraphElement>(null)
  const copyId = useId()

  useLayoutEffect(() => {
    const element = copy.current
    if (!element) return
    const measure = () => {
      const lineHeight = Number.parseFloat(getComputedStyle(element).lineHeight)
      setOverflows(element.scrollHeight > lineHeight * 3 + 1)
    }
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(element)
    return () => observer.disconnect()
  }, [description, expanded])

  useEffect(() => {
    if (!ready) return
    const controller = new AbortController()
    // Skip fleeting pin selections and StrictMode's initial effect remount.
    const timer = window.setTimeout(() => {
      fetchDescription(point, controller.signal)
        .then(text => { if (!controller.signal.aborted) setDescription(text) })
        .catch((reason: unknown) => {
          if (!controller.signal.aborted) setError(reason instanceof Error ? reason.message : 'Unable to describe this view.')
        })
    }, 150)
    return () => { window.clearTimeout(timer); controller.abort() }
  }, [point, attempt, ready])

  return <div className="panorama-description" aria-live="polite" aria-busy={!description && !error}>
    <div className="description-heading"><span>✦</span><h3>Your surroundings</h3></div>
    {description ? <>
      <p ref={copy} id={copyId} className={`description description-copy${expanded ? ' expanded' : ''}`}>{description}</p>
      {overflows && <button className="description-toggle" type="button" aria-expanded={expanded} aria-controls={copyId} onClick={() => setExpanded(value => !value)}>{expanded ? 'Read less' : 'Read more'}</button>}
    </> : error ? <>
      <p className="description description-copy expanded">{error}</p>
      <button className="retry-button" type="button" onClick={() => { setError(null); setAttempt(value => value + 1) }}>Retry description ↗</button>
    </> : <div className="description-pending" role="status"><p><span className="spinner" aria-hidden="true" />{ready ? 'Describing your surroundings…' : 'Waiting for your panorama…'}</p><div className="description-skeleton" aria-hidden="true"><span /><span /></div></div>}
  </div>
}
