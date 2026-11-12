import { useMeetingConfigStore } from '@/stores/meetingConfig'
import { SupportedVideoCodec } from '@/types/video-call'
import { storeToRefs } from 'pinia'

export function mediaErrorMessage(error: unknown, kind: 'camera' | 'microphone') {
  const label = kind === 'camera' ? 'Camera' : 'Microphone'

  if (error instanceof DOMException) {
    if (error.name === 'NotAllowedError') return `${label} permission was denied.`
    if (error.name === 'NotFoundError') return `No ${kind} was found.`
    if (error.name === 'NotReadableError') return `${label} is already in use by another app.`
  }

  return `Unable to access the ${kind}.`
}

const LAYER_SHARE: Record<string, number> = { q: 0.2, h: 0.6, f: 1 }
export type BitrateSetting = { mode: 'keep' } | { mode: 'unlimited' } | { mode: 'cap'; bps: number }

export const resolveBitrate = (v: unknown): BitrateSetting => {
  switch (v) {
    case 'auto':
      return { mode: 'keep' }
    case '1':
      return { mode: 'cap', bps: 1_000_000 }
    case '2':
      return { mode: 'cap', bps: 2_000_000 }
    case '4':
      return { mode: 'cap', bps: 4_000_000 }
    case '8':
      return { mode: 'cap', bps: 8_000_000 }
    default:
      return { mode: 'unlimited' }
  }
}

export const preferCodec = async (
  pc: RTCPeerConnection,
  codec: SupportedVideoCodec,
  kind: string = 'video',
): Promise<void> => {
  const form = useMeetingConfigStore().form
  codec ??= SupportedVideoCodec.VP9
  console.log(`sellected = ${kind}/${codec.toString()}`)
  if (typeof RTCRtpSender.getCapabilities !== 'function') return

  const codecs = RTCRtpSender.getCapabilities(kind)?.codecs ?? []

  const sellectedCodec = codecs.filter(
    (c) => c.mimeType.toLowerCase() === `${kind}/${codec.toString().toLowerCase()}`,
  )
  if (sellectedCodec.length === 0) {
    console.warn(`${kind}/${codec.toString()} is not available, recording will not work`)
    return
  }

  const bitrate = resolveBitrate(form.maxBitrate)

  if (bitrate.mode !== 'keep') {
    for (const s of pc.getSenders()) {
      if (s.track?.kind !== 'video') continue

      const p = s.getParameters()
      if (!p.encodings || p.encodings.length === 0) continue

      for (const enc of p.encodings) {
        if (bitrate.mode === 'unlimited') {
          delete enc.maxBitrate
        } else {
          const share = (enc.rid && LAYER_SHARE[enc.rid]) || 1
          enc.maxBitrate = Math.round(bitrate.bps * share)
        }
      }

      try {
        await s.setParameters(p)
      } catch (err) {
        console.warn('setParameters failed', err)
      }
    }
  }

  for (const transceiver of pc.getTransceivers()) {
    if ('stopped' in transceiver && transceiver.stopped) continue
    const trKind = transceiver.sender.track?.kind ?? transceiver.receiver.track?.kind
    if (trKind !== 'video') continue

    try {
      transceiver.setCodecPreferences(sellectedCodec)
    } catch (err) {
      console.warn('setCodecPreferences failed', err)
    }
  }
}

export const isSvcSupported = async () => {
  try {
    const info = await navigator.mediaCapabilities.encodingInfo({
      type: 'webrtc',
      video: {
        contentType: 'video/VP9',
        width: 1280,
        height: 720,
        bitrate: 2_500_000,
        framerate: 30,
        scalabilityMode: 'L3T3',
      },
    })
    return info.supported
  } catch {
    return false
  }
}
