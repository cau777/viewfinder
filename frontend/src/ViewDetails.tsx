import type { SunInfo, View } from './api'

function distance(metres: number) {
  return metres < 1000 ? `${Math.round(metres)} m` : `${(metres / 1000).toFixed(1)} km`
}
function SunCard({ title, sun }: { title: string; sun?: SunInfo }) {
  const time = sun ? new Intl.DateTimeFormat('en-CA', { timeZone: 'America/Vancouver', hour: '2-digit', minute: '2-digit', hourCycle: 'h23' }).format(new Date(sun.time)) : '—'
  return <div className="sun-card"><span className="sun-icon" aria-hidden="true">{title === 'Sunrise' ? '☀ ↗' : '☀ ↘'}</span><h4>{title}</h4><strong>{time}</strong>
    {sun && <><p>{Math.round(sun.bearing)}° from north</p><p className="sun-open">{Math.round(sun.open_share * 100)}% of directions open</p></>}
  </div>
}

export function ViewSummary({ view }: { view: View }) {
  const analysis = view.analysis
  const hits = view.points.filter(point => !point.unobstructed)
  const farthest = view.farthest_distance ?? hits.reduce((maximum, point) => Math.max(maximum, point.distance), 0)
  const open = view.unobstructed_share ?? (view.points.length ? 1 - hits.length / view.points.length : 0)
  const score = analysis ? Math.round(analysis.beauty_score) : null
  return <>
    <div className="beauty-card">
      <span className="eyebrow">BEAUTY SCORE</span>
      <div className="beauty-result"><div className="beauty-value"><strong>{score ?? '—'}</strong><span>/ 100</span></div>
        {score !== null && <div className="beauty-ring" style={{ background: `conic-gradient(var(--color-accent) ${score}%, #ffffff12 ${score}% 100%)` }} aria-hidden="true"><span>{score}%</span></div>}
      </div>
      <p>{analysis ? 'Openness, water, landmarks & sunlight' : 'Analysis unavailable for this view.'}</p>
    </div>
    <dl className="view-stats"><div><dt>Farthest sightline</dt><dd>{distance(farthest)}</dd></div><div><dt>Open directions</dt><dd>{Math.round(open * 100)}%</dd></div></dl>
  </>
}

export default function ViewDetails({ view }: { view: View }) {
  const analysis = view.analysis
  return <>
    <h3 className="view-section-heading">Sunrise & sunset</h3>
    <div className="sun-grid"><SunCard title="Sunrise" sun={analysis?.sunrise} /><SunCard title="Sunset" sun={analysis?.sunset} /></div>
    {analysis && <p className="analysis-date">{analysis.date} · Vancouver time</p>}
    <h3 className="view-section-heading">Ocean</h3>
    <div className="ocean-card"><span>Overlap with view area</span><strong>{analysis ? `${(analysis.ocean_area / 1_000_000).toLocaleString(undefined, { maximumFractionDigits: 3 })} km²` : '—'}</strong></div>
    <h3 className="view-section-heading">Landmarks in the view area{analysis ? ` · ${analysis.landmarks.length}` : ''}</h3>
    {analysis ? analysis.landmarks.length ? <ul className="landmark-list">{analysis.landmarks.map((name, index) => <li key={`${name}-${index}`}>{name}</li>)}</ul> : <p className="detail-note">No landmarks intersect this view area.</p> : <p className="detail-note">Landmark analysis unavailable.</p>}
    <details className="view-method"><summary>Elevation & method</summary>
      <dl className="method-stats"><div><dt>Ground altitude</dt><dd>{view.ground_altitude.toFixed(1)} m</dd></div><div><dt>Eye altitude</dt><dd>{view.altitude.toFixed(1)} m</dd></div><div><dt>Height above surface</dt><dd>{(view.altitude - view.ground_altitude).toFixed(1)} m</dd></div></dl>
      <p className="detail-note">Horizontal LiDAR rays approximate visibility at eye level. Open directions leave the dataset without a collision; visibility beyond its coverage is unknown.</p>
      <p className="detail-note">Ocean and landmark entries come from overlap with the calculated view polygon, rather than independently verified visibility. Sun percentages measure open rays around the sun’s direction. Beauty score combines area, water, landmarks and sunlight.</p>
      {!analysis && <p className="detail-note">Geographic analysis datasets are unavailable on this server.</p>}
    </details>
  </>
}
