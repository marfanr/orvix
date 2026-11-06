<script setup lang="ts">
import { type ComponentPublicInstance, computed, nextTick, ref, watch } from 'vue'
import type { Participant } from '@/types/video-call'
import { MicIcon, MicOffIcon } from 'lucide-vue-next'
import VideoMaskCanvas from '@/components/video-call/VideoMaskCanvas.vue'
import { useMeetingConfigStore } from '@/stores/meetingConfig.ts'
import { storeToRefs } from 'pinia'
import { useCallStore } from '@/stores/call.ts'

const meetingStore = useMeetingConfigStore()
const { form } = storeToRefs(meetingStore)
const callStore = useCallStore()
const { localRawStream } = storeToRefs(callStore)


const LOCAL_PARTICIPANT_ID = 0
const model = ref<{
  mask_ready: boolean
}>({
  mask_ready: false,
})

const props = defineProps<{
  participant: Participant
  tileClass?: string
}>()

const showCanvas = computed(
  () => props.participant.id === LOCAL_PARTICIPANT_ID && form.value.bgremoval != 'disabled',
)

const MicStateIcon = computed(() => (props.participant.muted ? MicOffIcon : MicIcon))

const videoRef = ref<HTMLVideoElement | null>(null)
const audioRef = ref<HTMLAudioElement | null>(null)

function attachStream(el: HTMLMediaElement, stream: MediaStream): void {
  if (el.srcObject !== stream) el.srcObject = stream
  el.play().catch((err) => console.warn(`${el.tagName.toLowerCase()} play error:`, err))
}

async function setVideoStream(): Promise<void> {
  await nextTick()
  const stream  = localRawStream.value
  if (videoRef.value && stream) attachStream(videoRef.value, stream)
}

async function setAudioStream(): Promise<void> {
  await nextTick()
  const { stream, muted } = props.participant
  if (!audioRef.value || !stream) return

  audioRef.value.muted = muted
  attachStream(audioRef.value, stream)
}

function setVideoRef(el: Element | ComponentPublicInstance | null): void {
  videoRef.value = el instanceof HTMLVideoElement ? el : null
  if (videoRef.value) setVideoStream()
}

function setAudioRef(el: Element | ComponentPublicInstance | null): void {
  audioRef.value = el instanceof HTMLAudioElement ? el : null
  if (audioRef.value) setAudioStream()
}

watch(() => props.participant.stream, setVideoStream)
watch([() => props.participant.muted, () => props.participant.stream], setAudioStream)
</script>

<template>
  <div
    class="relative overflow-hidden rounded-sm min-w-0 min-h-0 h-full w-full bg-stone-900 text-white flex items-center justify-center"
    :class="tileClass"
  >
    <div
      v-if="participant.cameraOff"
      class="size-16 sm:size-20 md:size-24 shrink-0 rounded-full bg-brown-ribbon-800 flex items-center justify-center"
    >
      <span class="text-white text-lg sm:text-xl md:text-2xl font-medium" aria-hidden="true">
        {{ participant.name[0]?.toUpperCase() }}
      </span>
    </div>

    <div v-else class="absolute inset-0">
      <VideoMaskCanvas
        v-if="showCanvas"
        :participant="participant"
        :video="videoRef"
        :model-value="model"
      />
      <video
        :ref="setVideoRef"
        class="w-full h-full bg-black object-cover"
        :class="{ 'opacity-0 pointer-events-none': showCanvas && !videoRef }"
        autoplay
        playsinline
        muted
      ></video>
    </div>

    <audio v-if="!showCanvas" :ref="setAudioRef" autoplay playsinline />

    <div
      class="absolute bottom-1 left-1 sm:bottom-2 sm:left-2 md:bottom-4 md:left-4 p-1 sm:p-1.5 md:p-2 text-[10px] sm:text-xs flex flex-row gap-x-1.5 sm:gap-x-2 md:gap-x-3 items-center bg-zinc-800/60 rounded-md text-white max-w-[calc(100%-0.5rem)] sm:max-w-[calc(100%-1rem)]"
    >
      <span class="truncate">{{ participant.name }}</span>
      <component
        :is="MicStateIcon"
        class="size-3 sm:size-3.5 md:size-[15px]"
        :aria-label="participant.muted ? 'Muted' : 'Unmuted'"
      />
    </div>
  </div>
</template>
