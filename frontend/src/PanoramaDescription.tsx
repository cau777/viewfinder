import { useEffect, useState } from 'react'
import { fetchDescription, type Coordinates } from './api'

/** Mounted for a single location only once its panorama has loaded. */
export default function PanoramaDescription({ point }: { point: Coordinates }) {
  const [description, setDescription] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [attempt, setAttempt] = useState(0)

  useEffect(() => {
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
  }, [point, attempt])

  return <div className="panorama-description" aria-live="polite" aria-busy={!description && !error}>
    <div className="description-heading"><span>✦</span><h3>Your surroundings</h3></div>
    {description ? <p className="description">{description}</p> : error ? <>
      <p className="description">{error}</p>
      <button className="retry-button" type="button" onClick={() => { setError(null); setAttempt(value => value + 1) }}>Retry description ↗</button>
    </> : <p className="description">Describing your surroundings…</p>}
  </div>
}
