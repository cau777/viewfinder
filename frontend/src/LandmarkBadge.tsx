type LandmarkKind = 'beach' | 'park' | 'bridge' | 'building' | 'water' | 'landmark'

// Compact dataset identifiers need explicit word boundaries and, where known, a category.
const knownLandmarks = new Map<string, { label: string; kind: LandmarkKind }>([
  ['englishbay', { label: 'English Bay', kind: 'beach' }],
  ['englishbaybeach', { label: 'English Bay Beach', kind: 'beach' }],
  ['stanleypark', { label: 'Stanley Park', kind: 'park' }],
  ['queenelizabethpark', { label: 'Queen Elizabeth Park', kind: 'park' }],
  ['kitsilanobeach', { label: 'Kitsilano Beach', kind: 'beach' }],
  ['sunsetbeach', { label: 'Sunset Beach', kind: 'beach' }],
  ['jerichobeach', { label: 'Jericho Beach', kind: 'beach' }],
  ['locarnobeach', { label: 'Locarno Beach', kind: 'beach' }],
  ['spanishbanks', { label: 'Spanish Banks', kind: 'beach' }],
  ['wreckbeach', { label: 'Wreck Beach', kind: 'beach' }],
  ['secondbeach', { label: 'Second Beach', kind: 'beach' }],
  ['thirdbeach', { label: 'Third Beach', kind: 'beach' }],
  ['canadaplace', { label: 'Canada Place', kind: 'building' }],
  ['scienceworld', { label: 'Science World', kind: 'building' }],
  ['harbourcentre', { label: 'Harbour Centre', kind: 'building' }],
  ['vancouverartgallery', { label: 'Vancouver Art Gallery', kind: 'building' }],
  ['lionsgatebridge', { label: 'Lions Gate Bridge', kind: 'bridge' }],
  ['burrardbridge', { label: 'Burrard Bridge', kind: 'bridge' }],
  ['granvillebridge', { label: 'Granville Bridge', kind: 'bridge' }],
  ['granvilleisland', { label: 'Granville Island', kind: 'water' }],
  ['falsecreek', { label: 'False Creek', kind: 'water' }],
  ['gastownsteamclock', { label: 'Gastown Steam Clock', kind: 'landmark' }],
])

function presentLandmark(name: string): { label: string; kind: LandmarkKind } {
  const known = knownLandmarks.get(name.toLowerCase().replace(/[^a-z0-9]/g, ''))
  if (known) return known
  const label = name.trim()
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2')
    .replace(/([a-z\d])([A-Z])/g, '$1 $2')
    .replace(/[_-]+/g, ' ')
    .replace(/^([a-z]+)(beach|park|bridge|museum|tower|lake|bay|island|gardens?)$/i, '$1 $2')
    .replace(/\s+/g, ' ')
    .split(' ')
    .map(word => word === word.toUpperCase() && word.length <= 4 ? word : word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
    .join(' ')
  const words = label.toLowerCase()
  const kind = /\b(beach|banks)\b/.test(words) ? 'beach'
    : /\b(park|garden|gardens)\b/.test(words) ? 'park'
    : /\bbridge\b/.test(words) ? 'bridge'
    : /\b(museum|gallery|tower|centre|center|building)\b/.test(words) ? 'building'
    : /\b(bay|creek|lake|river|harbour|harbor|island)\b/.test(words) ? 'water'
    : 'landmark'
  return { label: label || 'Unnamed landmark', kind }
}

const iconPaths: Record<LandmarkKind, string> = {
  beach: 'M3 11a9 9 0 0 1 18 0H3Zm9-9v17m-4 2h8M12 2c-3 2-4 5-4 9m4-9c3 2 4 5 4 9',
  park: 'm12 3-5 6h3l-5 6h5v6h4v-6h5l-5-6h3l-5-6Z',
  bridge: 'M3 17h18M6 5v16M18 5v16M6 7c4 8 8 8 12 0M9 11v6m3-4v4m3-6v6',
  building: 'M4 21h16M6 21V7h12v14M9 7V3h6v4M9 11h1m4 0h1m-6 4h1m4 0h1m-4 6v-3h2v3',
  water: 'M3 7c2-2 4 2 6 0s4 2 6 0 4 2 6 0M3 12c2-2 4 2 6 0s4 2 6 0 4 2 6 0M3 17c2-2 4 2 6 0s4 2 6 0 4 2 6 0',
  landmark: 'M12 21s7-7 7-12a7 7 0 0 0-14 0c0 5 7 12 7 12Zm0-15a3 3 0 1 0 0 6 3 3 0 0 0 0-6Z',
}

export default function LandmarkBadge({ name }: { name: string }) {
  const { label, kind } = presentLandmark(name)
  return <li className={`landmark-badge landmark-${kind}`}>
    <svg className="landmark-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={iconPaths[kind]} /></svg>
    <span>{label}</span>
  </li>
}
