export type HistogramEntry = { byte: number; char: string | null; count: number }

export type AnalyzeRequest = { text: string; repeat: number; capacity: number }

export type AnalyzeResponse = {
  length: number
  capacity: number
  checksum: number
  hex_preview: string
  histogram_top: HistogramEntry[]
  core_version: string
}

// Relative URL: in dev Vite proxies /api to uvicorn; in prod FastAPI serves both.
export async function analyze(req: AnalyzeRequest): Promise<AnalyzeResponse> {
  const res = await fetch('/api/analyze', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  })
  const body = await res.json().catch(() => ({ detail: res.statusText }))
  if (!res.ok) {
    const detail = typeof body.detail === 'string' ? body.detail : JSON.stringify(body.detail)
    throw new Error(`${res.status}: ${detail}`)
  }
  return body as AnalyzeResponse
}
