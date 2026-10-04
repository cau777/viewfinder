export type Coordinates = { latitude: number; longitude: number }
export type ViewDescription = { title: string; description: string; tags: string[] }

// Replace this adapter with a POST to the view API when it is available.
// Its coordinate input and AbortSignal can be passed directly to fetch.
export async function describeView(point: Coordinates, signal: AbortSignal): Promise<ViewDescription> {
  await new Promise<void>((resolve, reject) => {
    const abort = () => { clearTimeout(timer); reject(new DOMException('Request cancelled', 'AbortError')) }
    const timer = setTimeout(() => { signal.removeEventListener('abort', abort); resolve() }, 1100)
    if (signal.aborted) abort()
    else signal.addEventListener('abort', abort, { once: true })
  })
  if (point.latitude > 49.3 && point.longitude < -123.13) return {
    title: 'Between forest & sea',
    description: 'Imagine a quiet coastal outlook: tall evergreens frame a stretch of water, with the North Shore mountains rising in the distance. Along the shoreline, walkers and cyclists follow the curve of the seawall. This sample evokes Vancouver’s forested waterfront; the actual view at your pin may differ.',
    tags: ['Coastal', 'Evergreens', 'Mountain backdrop'],
  }
  if (point.latitude < 49.278 && point.longitude < -123.13) return {
    title: 'A waterfront perspective',
    description: 'Picture the water opening up in front of you, broken by small boats and reflections of the city skyline. The shoreline brings together low buildings, waterfront paths, and pockets of greenery. This is an imagined Vancouver waterfront scene, rather than a verified view from this spot.',
    tags: ['Waterfront', 'City skyline', 'Open skies'],
  }
  return {
    title: 'The city, from here',
    description: 'Picture Vancouver’s glass towers catching the light above a lively streetscape. Trees soften the edges of the city, and between the buildings you might glimpse the mountains that surround it. This illustrative scene captures the feel of Vancouver; a future location service will describe the actual surroundings at your pin.',
    tags: ['Urban', 'Architecture', 'Mountain backdrop'],
  }
}

export type AddressResult = Coordinates & { label: string }

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
