import { BatchSpanProcessor, WebTracerProvider } from '@opentelemetry/sdk-trace-web'
import { OTLPTraceExporter } from '@opentelemetry/exporter-trace-otlp-http'
import { resourceFromAttributes } from '@opentelemetry/resources'
import { ZoneContextManager } from '@opentelemetry/context-zone'
import { registerInstrumentations } from '@opentelemetry/instrumentation'
import { FetchInstrumentation } from '@opentelemetry/instrumentation-fetch'

export const tracerName = 'vidcall-frontend'

let initialized = false

const otlpTraceUrl = (): string => {
  const configured = import.meta.env.VITE_OTEL_EXPORTER_OTLP_ENDPOINT?.trim()
  if (!configured) return 'http://localhost:4318/v1/traces'
  return `${configured.replace(/\/v1\/traces\/?$/, '')}/v1/traces`
}

export function initTelemetry(): void {
  if (import.meta.env.VITE_OTEL_ENABLED !== 'true') return
  if (initialized) return
  initialized = true

  const provider = new WebTracerProvider({
    resource: resourceFromAttributes({
      'service.name': 'vidcall-frontend',
    }),
    spanProcessors: [
      new BatchSpanProcessor(
        new OTLPTraceExporter({
          url: otlpTraceUrl(),
        }),
      ),
    ],
  })

  provider.register({ contextManager: new ZoneContextManager() })

  registerInstrumentations({
    instrumentations: [
      new FetchInstrumentation({
        propagateTraceHeaderCorsUrls: [/localhost:8080/],
        ignoreUrls: [/\/v1\/traces/],
      }),
    ],
  })

  console.info('[otel] initialized →', otlpTraceUrl())
}
