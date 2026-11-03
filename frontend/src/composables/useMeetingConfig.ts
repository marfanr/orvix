import { inject, type InjectionKey, provide, reactive } from 'vue'
import { SupportedBitrate, SupportedVideoCodec } from '@/types/video-call.ts'

export type MeetingConfigSectionKey = 'network' | 'video' | 'audio' | 'recording' | 'advanced'

export interface MeetingConfigForm {
  maxBitrate: string
  preferredCodec: SupportedVideoCodec
  networkPriority: string
  iceTransport: string
  adaptiveBitrate: boolean
  simulcast: boolean
  camera: string
  resolution: string
  framerate: string
  contentHint: string
  mirrorPreview: boolean
  lowLightMode: boolean
  mic: string
  audioBitrate: string
  noiseSuppression: boolean
  echoCancellation: boolean
  autoGain: boolean
  stereo: boolean
  recordingEnabled: boolean
  recordingNotify: boolean
  recordingLayout: string
  recordingQuality: string
  recordingFormat: string
  recordVideo: boolean
  recordAudio: boolean
  recordChat: boolean
  stunUrl: string
  turnUrl: string
  forceRelay: boolean
  diagStats: boolean
  logSdp: boolean
  logIce: boolean
  bitrateGraph: boolean
}

export type MeetingConfigToggleKey =
  | 'adaptiveBitrate'
  | 'simulcast'
  | 'mirrorPreview'
  | 'lowLightMode'
  | 'noiseSuppression'
  | 'echoCancellation'
  | 'autoGain'
  | 'stereo'
  | 'recordingEnabled'
  | 'recordingNotify'
  | 'forceRelay'
  | 'diagStats'

export interface MeetingConfigToggleItem {
  key: MeetingConfigToggleKey
  label: string
  description: string
}

export const MEETING_CONFIG_DEFAULTS: MeetingConfigForm = {
  maxBitrate: SupportedBitrate.B1,
  preferredCodec: SupportedVideoCodec.VP9,
  networkPriority: 'high',
  iceTransport: 'all',
  adaptiveBitrate: true,
  simulcast: true,
  camera: '',
  resolution: '720p',
  framerate: '30',
  contentHint: 'motion',
  mirrorPreview: true,
  lowLightMode: false,
  mic: '',
  audioBitrate: '64',
  noiseSuppression: true,
  echoCancellation: true,
  autoGain: false,
  stereo: false,
  recordingEnabled: false,
  recordingNotify: true,
  recordingLayout: 'grid',
  recordingQuality: 'call',
  recordingFormat: 'mp4',
  recordVideo: true,
  recordAudio: true,
  recordChat: false,
  stunUrl: 'stun:stun.l.google.com:19302',
  turnUrl: '',
  forceRelay: false,
  diagStats: false,
  logSdp: false,
  logIce: false,
  bitrateGraph: true,
}

export interface MeetingConfigContext {
  form: MeetingConfigForm
  resetDefaults: () => void
  save: () => void
}

const MEETING_CONFIG_KEY: InjectionKey<MeetingConfigContext> = Symbol('meeting-config')

export function provideMeetingConfig(): MeetingConfigContext {

  let config = MEETING_CONFIG_DEFAULTS

  const _saved = JSON.parse(localStorage.getItem("meeting-config") ?? "{}")
  if (_saved) {
    config = { ..._saved }
  }
  const form = reactive<MeetingConfigForm>({ ...config })

  const context: MeetingConfigContext = {
    form,
    resetDefaults: () => {
      Object.assign(form, config)
    },
    save: () => {
      localStorage.setItem("meeting-config", JSON.stringify(form))
    }
  }

  provide(MEETING_CONFIG_KEY, context)
  return context
}

export function useMeetingConfig(): MeetingConfigContext {
  const context = inject(MEETING_CONFIG_KEY)
  if (!context) {
    throw new Error(
      'useMeetingConfig() must be called inside a component that provides meeting config (a descendant of the provider)',
    )
  }
  return context
}
