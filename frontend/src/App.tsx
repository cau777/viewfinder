import {
  Alert,
  Badge,
  Box,
  Button,
  Code,
  Container,
  Grid,
  Group,
  NumberInput,
  Paper,
  RingProgress,
  SimpleGrid,
  Stack,
  Text,
  Textarea,
  Title,
} from '@mantine/core'
import { useState, type FormEvent, type ReactNode } from 'react'
import { analyze, type AnalyzeResponse } from './api.ts'

const PIPELINE = ['React', 'FastAPI', 'Python', 'Rust · Vec<u8>']

function StatCard({ label, children, delay = 0 }: { label: string; children: ReactNode; delay?: number }) {
  return (
    <Paper className="glass pop-in" style={{ animationDelay: `${delay}ms` }}>
      <Text size="xs" c="dimmed" tt="uppercase" fw={700} lts={1.5}>
        {label}
      </Text>
      <Box mt="xs">{children}</Box>
    </Paper>
  )
}

export default function App() {
  const [text, setText] = useState('Hello from React → FastAPI → Rust! 🦀')
  const [repeat, setRepeat] = useState<number | string>(3)
  const [capacity, setCapacity] = useState<number | string>(4096)
  const [result, setResult] = useState<AnalyzeResponse | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)

  async function onSubmit(e: FormEvent) {
    e.preventDefault()
    setLoading(true)
    setError(null)
    try {
      setResult(await analyze({ text, repeat: Number(repeat), capacity: Number(capacity) }))
    } catch (err) {
      setResult(null)
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }

  const fill = result ? Math.min(100, (result.length / result.capacity) * 100) : 0
  const maxCount = result ? Math.max(...result.histogram_top.map((h) => h.count)) : 1

  return (
    <Container size="lg" py={60}>
      <Stack gap={40}>
        <Stack gap="sm" align="center" ta="center">
          <Group gap="xs">
            {PIPELINE.map((step, i) => (
              <Group key={step} gap="xs">
                <Badge size="lg" variant={i === PIPELINE.length - 1 ? 'gradient' : 'outline'} color="neon.3">
                  {step}
                </Badge>
                {i < PIPELINE.length - 1 && <Text c="cyber.4">→</Text>}
              </Group>
            ))}
          </Group>
          <Title className="gradient-text" fz={72} lh={1.05}>
            viewfinder
          </Title>
          <Text c="gray.4" size="lg" maw={640}>
            Your text gets pushed into a Rust-owned <Code>Vec&lt;u8&gt;</Code> through PyO3, crunched, and
            beamed back.
          </Text>
        </Stack>

        <Grid gap="xl" align="stretch">
          <Grid.Col span={5}>
            <Paper component="form" onSubmit={onSubmit} h="100%">
              <Stack>
                <Textarea label="Payload" value={text} onChange={(e) => setText(e.currentTarget.value)} autosize minRows={4} />
                <SimpleGrid cols={2}>
                  <NumberInput label="Repeat" min={1} max={100} value={repeat} onChange={setRepeat} />
                  <NumberInput label="Capacity (bytes)" min={1} value={capacity} onChange={setCapacity} />
                </SimpleGrid>
                <Button type="submit" size="lg" loading={loading} fullWidth mt="sm">
                  Crunch it in Rust ⚡
                </Button>
                {error && (
                  <Alert color="pink" variant="light" title="Rust said no">
                    {error}
                  </Alert>
                )}
              </Stack>
            </Paper>
          </Grid.Col>

          <Grid.Col span={7}>
            {result ? (
              // key forces the pop-in animation to replay on each new result
              <SimpleGrid cols={2} spacing="lg" key={`${result.checksum}-${result.length}`}>
                <StatCard label="Buffer fill">
                  <Group>
                    <RingProgress
                      size={96}
                      thickness={10}
                      roundCaps
                      sections={[{ value: fill, color: fill > 90 ? 'pink' : 'cyber.5' }]}
                      label={<Text ta="center" fw={700}>{fill.toFixed(1)}%</Text>}
                    />
                    <div>
                      <Text fz={28} fw={700}>{result.length}</Text>
                      <Text size="sm" c="dimmed">of {result.capacity} bytes</Text>
                    </div>
                  </Group>
                </StatCard>
                <StatCard label="Adler-32 checksum" delay={60}>
                  <Text ff="monospace" fz={30} fw={600} className="gradient-text">
                    0x{result.checksum.toString(16).padStart(8, '0')}
                  </Text>
                  <Text size="sm" c="dimmed" mt={4}>core v{result.core_version}</Text>
                </StatCard>
                <Box style={{ gridColumn: 'span 2' }}>
                  <StatCard label="Top bytes" delay={120}>
                    <Stack gap={8}>
                      {result.histogram_top.map((h) => (
                        <Group key={h.byte} gap="sm" wrap="nowrap">
                          <Code w={56} ta="center">{h.char !== null ? `'${h.char}'` : `0x${h.byte.toString(16)}`}</Code>
                          <Box
                            h={10}
                            style={{
                              width: `${(h.count / maxCount) * 100}%`,
                              borderRadius: 99,
                              background: 'linear-gradient(90deg, var(--mantine-color-neon-5), var(--mantine-color-cyber-5))',
                              boxShadow: '0 0 12px rgba(8, 208, 255, 0.5)',
                            }}
                          />
                          <Text size="sm" fw={700}>{h.count}</Text>
                        </Group>
                      ))}
                    </Stack>
                  </StatCard>
                </Box>
                <Box style={{ gridColumn: 'span 2' }}>
                  <StatCard label="Hex preview" delay={180}>
                    <Code block c="cyber.3" style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-all' }}>
                      {result.hex_preview || '—'}
                    </Code>
                  </StatCard>
                </Box>
              </SimpleGrid>
            ) : (
              <Paper h="100%" style={{ display: 'grid', placeItems: 'center', minHeight: 320 }}>
                <Stack align="center" gap="xs">
                  <Text fz={56}>🦀</Text>
                  <Text c="dimmed">Hit the button. Results land here.</Text>
                </Stack>
              </Paper>
            )}
          </Grid.Col>
        </Grid>
      </Stack>
    </Container>
  )
}
