import { defineStore } from 'pinia'
import { reactive } from 'vue'
import { SupportedBitrate, SupportedVideoCodec } from '@/types/video-call'

export type MeetingConfigSectionKey = 'network' | 'video' | 'audio' | 'recording' | 'advanced'

export type BgRemovalKey = 'disabled' | 'blur' | 'image' | 'color'

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

export interface MeetingConfigForm {
  maxBitrate: SupportedBitrate
  preferredCodec: SupportedVideoCodec
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
  bgremoval: BgRemovalKey
  blurRadius: number
}

export const MEETING_CONFIG_DEFAULTS: MeetingConfigForm = {
  maxBitrate: SupportedBitrate.B1,
  preferredCodec: SupportedVideoCodec.VP9,
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
  bgremoval: 'disabled',
  blurRadius: 0,
}

export const useMeetingConfigStore = defineStore('meetingConfig', () => {
  const _saved = JSON.parse(localStorage.getItem('meeting-config') ?? '{}')

  const form = reactive<MeetingConfigForm>({
    ...MEETING_CONFIG_DEFAULTS,
    ..._saved,
  })

  function save() {
    localStorage.setItem('meeting-config', JSON.stringify(form))
  }

  function resetDefaults() {
    Object.assign(form, MEETING_CONFIG_DEFAULTS)
  }

  return {
    form,
    save,
    resetDefaults,
  }
})
