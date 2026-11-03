import { onMounted, onUnmounted, ref, watch } from 'vue'
import { mediaErrorMessage } from '@/lib/media'
import type { MeetingConfigForm } from '@/stores/meetingConfig'

export function useMicrophoneTest(form: MeetingConfigForm) {
  const audioInputs = ref<MediaDeviceInfo[]>([])
  const micStream = ref<MediaStream | null>(null)
  const micError = ref('')
  const micLoading = ref(false)
  const micLevel = ref(0)

  let micRequest = 0
  let audioContext: AudioContext | null = null
  let analyser: AnalyserNode | null = null
  let micFrame: number | null = null

  async function refreshDevices() {
    if (!navigator.mediaDevices?.enumerateDevices) return

    try {
      const devices = await navigator.mediaDevices.enumerateDevices()
      audioInputs.value = devices.filter((device) => device.kind === 'audioinput')
    } catch {
      audioInputs.value = []
    }
  }

  onMounted(() => {
    void refreshDevices()
  })

  function stopMicTest() {
    micRequest += 1

    if (micFrame !== null) cancelAnimationFrame(micFrame)
    micFrame = null

    micStream.value?.getTracks().forEach((track) => track.stop())
    micStream.value = null

    analyser = null
    void audioContext?.close().catch(() => undefined)
    audioContext = null

    micLevel.value = 0
  }

  async function startMicTest() {
    stopMicTest()

    micError.value = ''
    micLoading.value = true

    if (!navigator.mediaDevices?.getUserMedia) {
      micError.value = 'Microphone access is not supported in this browser.'
      micLoading.value = false
      return
    }

    const request = micRequest

    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          ...(form.mic ? { deviceId: { ideal: form.mic } } : {}),
          noiseSuppression: form.noiseSuppression,
          echoCancellation: form.echoCancellation,
          autoGainControl: form.autoGain,
        },
        video: false,
      })

      if (request !== micRequest) {
        stream.getTracks().forEach((track) => track.stop())
        return
      }

      micStream.value = stream
      await refreshDevices()

      const context = new AudioContext()
      audioContext = context

      if (context.state === 'suspended') await context.resume()

      const source = context.createMediaStreamSource(stream)
      const node = context.createAnalyser()
      node.fftSize = 512
      node.smoothingTimeConstant = 0.7
      analyser = node

      source.connect(node)

      const samples = new Uint8Array(node.fftSize)

      const tick = () => {
        if (!analyser) return

        analyser.getByteTimeDomainData(samples)

        let sum = 0
        for (const sample of samples) {
          const value = (sample - 128) / 128
          sum += value * value
        }

        const rms = Math.sqrt(sum / samples.length)
        micLevel.value = Math.min(100, Math.round(rms * 250))

        micFrame = requestAnimationFrame(tick)
      }

      tick()
    } catch (error) {
      micError.value = mediaErrorMessage(error, 'microphone')
    } finally {
      if (request === micRequest) micLoading.value = false
    }
  }

  watch(
    () => [form.mic, form.noiseSuppression, form.echoCancellation, form.autoGain],
    () => {
      if (micStream.value) void startMicTest()
    },
  )

  onUnmounted(stopMicTest)

  return {
    audioInputs,
    micStream,
    micError,
    micLoading,
    micLevel,
    startMicTest,
    stopMicTest,
  }
}
