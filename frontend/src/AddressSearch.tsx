import { useCallback, useEffect, useRef, useState, type FormEvent } from 'react'
import { searchAddresses, type AddressResult } from './api'

export default function AddressSearch({ onSelect }: { onSelect: (result: AddressResult) => void }) {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<AddressResult[]>([])
  const [loading, setLoading] = useState(false)
  const [message, setMessage] = useState('')
  const [open, setOpen] = useState(false)
  const controller = useRef<AbortController | null>(null)
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const selectedLabel = useRef<string | null>(null)
  const root = useRef<HTMLFormElement>(null)

  useEffect(() => {
    const dismiss = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false)
    }
    document.addEventListener('pointerdown', dismiss)
    return () => { document.removeEventListener('pointerdown', dismiss); controller.current?.abort() }
  }, [])

  const runSearch = useCallback(async () => {
    if (query.trim().length < 3) { setMessage('Enter at least 3 characters.'); setOpen(true); return }
    controller.current?.abort()
    const request = new AbortController()
    controller.current = request
    setLoading(true)
    setOpen(true)
    setResults([])
    setMessage('')
    try {
      const addresses = await searchAddresses(query.trim(), request.signal)
      if (request.signal.aborted) return
      setResults(addresses)
      if (!addresses.length) setMessage('No Vancouver-area addresses found. Try a street number or place name.')
    } catch {
      if (!request.signal.aborted) setMessage('Address search is unavailable. Please try again.')
    } finally {
      if (!request.signal.aborted) setLoading(false)
    }
  }, [query])

  useEffect(() => {
    controller.current?.abort()
    setResults([])
    setMessage('')
    setLoading(false)
    if (query.trim().length < 3 || query === selectedLabel.current) {
      setOpen(false)
      return
    }
    setLoading(true)
    setOpen(true)
    timer.current = setTimeout(() => { void runSearch() }, 350)
    return () => {
      if (timer.current) clearTimeout(timer.current)
      controller.current?.abort()
    }
  }, [query, runSearch])

  function submit(event: FormEvent) {
    event.preventDefault()
    if (timer.current) clearTimeout(timer.current)
    void runSearch()
  }

  return <form ref={root} className="address-search" onSubmit={submit} role="search" onKeyDown={event => { if (event.key === 'Escape') setOpen(false) }}>
    <div className="search-field">
      <span aria-hidden="true">⌕</span>
      <input type="search" aria-label="Search for an address or place" placeholder="Search Vancouver addresses" value={query}
        onChange={event => { controller.current?.abort(); selectedLabel.current = null; setQuery(event.target.value) }}
        onFocus={() => { if (results.length || message) setOpen(true) }} />
      <button type="submit" aria-label="Search addresses" aria-expanded={open} aria-controls="address-results">Search</button>
    </div>
    {open && <div className="search-results" id="address-results">
      <div className="search-status" role="status">{loading ? 'Searching addresses…' : message || 'Select a location'}</div>
      {results.length > 0 && <ul>{results.map((result, index) => <li key={`${result.latitude}-${result.longitude}-${index}`}><button type="button" onClick={() => { controller.current?.abort(); selectedLabel.current = result.label; onSelect(result); setQuery(result.label); setOpen(false) }}><span aria-hidden="true">⌖</span><span>{result.label}</span><span aria-hidden="true">↗</span></button></li>)}</ul>}
      <a className="search-attribution" href="https://www.openstreetmap.org/copyright" target="_blank" rel="noreferrer">Search by Photon · © OpenStreetMap contributors</a>
    </div>}
  </form>
}
