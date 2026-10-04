export type Coordinates = { latitude: number; longitude: number }
export type ViewPoint = Coordinates & {
  /** Degrees clockwise from north */
  bearing: number
  /** Metres from the observer */
  distance: number
  /** The ray hit nothing; its distance is limited to the average sightline */
  unobstructed: boolean
}
export type SunInfo = { time: string; bearing: number; open_share: number }
export type ViewAnalysis = {
  date: string
  beauty_score: number
  sunrise: SunInfo
  sunset: SunInfo
  ocean_area: number
  lake_area: number
  water_area: number
  openness_area: number
  landmarks: string[]
}
export type View = Coordinates & {
  ground_altitude: number
  /** Altitude of the observer's eyes, metres */
  altitude: number
  /** Mean distance of the rays that hit the surface, metres */
  average_distance: number
  /** Statistics from all rays before polygon vertices are thinned. */
  farthest_distance?: number
  unobstructed_share?: number
  /** Where each ray around the observer ends: the vertices of the visible area */
  points: ViewPoint[]
  analysis?: ViewAnalysis | null
}

export async function fetchView(point: Coordinates, signal: AbortSignal): Promise<View> {
  const response = await fetch('/api/view', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(point),
    signal,
  })
  const body = await response.json().catch(() => ({ detail: response.statusText }))
  if (!response.ok) throw new Error(typeof body.detail === 'string' ? body.detail : 'Unable to load this view.')
  return body as View
}

export type AddressResult = Coordinates & { label: string }

export async function fetchDescription(point: Coordinates, signal: AbortSignal): Promise<string> {
  const response = await fetch('/api/panorama/description', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(point),
    signal,
  })
  const body = await response.json().catch(() => ({ detail: response.statusText }))
  if (!response.ok) throw new Error(typeof body.detail === 'string' ? body.detail : 'Unable to describe this view.')
  return body.description
}

// Vancouver and its immediate surroundings, in west/south/east/north order.
const VANCOUVER_SEARCH_BOUNDS = [-123.30, 49.19, -123.00, 49.33] as const

export async function searchAddresses(query: string, signal: AbortSignal): Promise<AddressResult[]> {
  const params = new URLSearchParams({ q: query, lat: '49.2827', lon: '-123.1207', limit: '5', lang: 'en', countrycode: 'CA', bbox: VANCOUVER_SEARCH_BOUNDS.join(','), location_bias_scale: '0.1' })
  const response = await fetch(`https://photon.komoot.io/api/?${params}`, { signal })
  if (!response.ok) throw new Error('Address search failed')
  const data = await response.json() as {
    features: { geometry: { coordinates: number[] }; properties: Record<string, string | number | undefined> }[]
  }
  return data.features.flatMap(({ geometry, properties }) => {
    const [longitude, latitude] = geometry.coordinates
    if (!Number.isFinite(latitude) || !Number.isFinite(longitude)) return []
    const [west, south, east, north] = VANCOUVER_SEARCH_BOUNDS
    if (longitude < west || longitude > east || latitude < south || latitude > north) return []
    const street = [properties.housenumber, properties.street].filter(Boolean).join(' ')
    const label = [...new Set([properties.name, street, properties.city ?? properties.town ?? properties.village, properties.state, properties.country].filter(Boolean))].join(', ')
    return [{ latitude, longitude, label: label || `${latitude.toFixed(5)}, ${longitude.toFixed(5)}` }]
  })
}

/** PNG of the full circle around a point, coloured by what each ray hits. Starts at north and turns
 * clockwise; the backend renders 4 pixels per degree, so its width covers 360° exactly. */
export function panoramaUrl(point: Coordinates): string {
  return `/api/panorama.png?${new URLSearchParams({ latitude: String(point.latitude), longitude: String(point.longitude), render: 'no-collision-v1' })}`
}
