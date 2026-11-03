import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { mediaErrorMessage } from '@/lib/media'
import type { MeetingConfigForm } from '@/stores/meetingConfig'

function resolutionHeight(resolution: string): number | undefined {
  switch (resolution) {
    case '360p':
      return 360
    case '720p':
      return 720
    case '1080p':
      return 1080
    default:
      return undefined
  }
}

export function useCameraPreview(form: MeetingConfigForm) {
  const videoInputs = ref<MediaDeviceInfo[]>([])
  const videoPreviewEl = ref<HTMLVideoElement | null>(null)
  const videoStream = ref<MediaStream | null>(null)
  const videoError = ref('')
  const videoLoading = ref(false)

  let previewRequest = 0

  async function refreshDevices() {
    if (!navigator.mediaDevices?.enumerateDevices) return

    try {
      const devices = await navigator.mediaDevices.enumerateDevices()
      videoInputs.value = devices.filter((device) => device.kind === 'videoinput')
    } catch {
      videoInputs.value = []
    }
  }

  onMounted(() => {
    void refreshDevices()
  })

  const activeCameraLabel = computed(() => {
    if (!form.camera) return 'Default camera'

    const device = videoInputs.value.find((input) => input.deviceId === form.camera)
    return device?.label || 'Selected camera'
  })

  function stopVideoPreview() {
    previewRequest += 1

    videoStream.value?.getTracks().forEach((track) => track.stop())
    videoStream.value = null

    if (videoPreviewEl.value) videoPreviewEl.value.srcObject = null
  }

  async function startVideoPreview() {
    stopVideoPreview()

    videoError.value = ''
    videoLoading.value = true

    if (!navigator.mediaDevices?.getUserMedia) {
      videoError.value = 'Camera access is not supported in this browser.'
      videoLoading.value = false
      return
    }

    const request = previewRequest
    const height = resolutionHeight(form.resolution)

    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        video: {
          ...(form.camera ? { deviceId: { ideal: form.camera } } : {}),
          ...(height ? { height: { ideal: height } } : {}),
          frameRate: { ideal: Number(form.framerate) || 30 },
        },
        audio: false,
      })

      if (request !== previewRequest) {
        stream.getTracks().forEach((track) => track.stop())
        return
      }

      videoStream.value = stream
      await refreshDevices()

      await nextTick()
      if (videoPreviewEl.value) {
        videoPreviewEl.value.srcObject = stream
        await videoPreviewEl.value.play().catch(() => undefined)
      }
    } catch (error) {
      videoError.value = mediaErrorMessage(error, 'camera')
    } finally {
      if (request === previewRequest) videoLoading.value = false
    }
  }

  watch(
    () => [form.camera, form.resolution, form.framerate],
    () => {
      if (videoStream.value) void startVideoPreview()
    },
  )

  onUnmounted(stopVideoPreview)

  return {
    videoInputs,
    videoPreviewEl,
    videoStream,
    videoError,
    videoLoading,
    activeCameraLabel,
    startVideoPreview,
    stopVideoPreview,
  }
}
