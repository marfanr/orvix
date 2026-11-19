<!--
SPDX-FileCopyrightText: 2026 Mohammad Arfan
SPDX-License-Identifier: AGPL-3.0
-->

<script setup lang="ts">
import type { Participant } from '@/types/video-call'
import { nextTick, onMounted, onBeforeUnmount, ref, watch, type ComponentPublicInstance } from 'vue'

const props = defineProps<{
  participant: Participant
}>()

const audioRef = ref<HTMLAudioElement | null>(null)
const audioUnlocked = ref(false)
const audioContext = new AudioContext()

let source: MediaStreamAudioSourceNode | null = null
let highPass: BiquadFilterNode | null = null
let lowPass: BiquadFilterNode | null = null
let gain: GainNode | null = null
let unmounted = false

function unlockAudio() {
  if (unmounted) return
  audioUnlocked.value = true
  audioContext.resume().catch(() => {})
  setAudioStream()
}

function audioPipeline(stream: MediaStream): MediaStream {
  source?.disconnect()
  highPass?.disconnect()
  lowPass?.disconnect()
  gain?.disconnect()

  source = audioContext.createMediaStreamSource(stream)

  highPass = audioContext.createBiquadFilter()
  highPass.type = 'highpass'
  highPass.frequency.value = 100
  highPass.Q.value = 0.6

  lowPass = audioContext.createBiquadFilter()
  lowPass.type = 'lowpass'
  lowPass.frequency.value = 10000

  gain = audioContext.createGain()
  gain.gain.value = 1.1

  const destination = audioContext.createMediaStreamDestination()

  source.connect(highPass)
  highPass.connect(lowPass)

  // Dry signal
  lowPass.connect(gain)

  gain.connect(destination)

  return destination.stream
}

async function setAudioStream(): Promise<void> {
  await nextTick()
  if (unmounted) return

  const { stream, muted, id } = props.participant
  const el = audioRef.value

  if (!el || !stream || id === 0) return

  el.muted = muted

  try {
    if (audioContext.state === 'suspended' && audioUnlocked.value) {
      await audioContext.resume()
    }

    const processedStream = audioPipeline(stream)
    el.srcObject = processedStream

    if (audioUnlocked.value) {
      await el.play()
    }
  } catch (err) {
    console.warn('Audio pipeline error:', err)
    el.srcObject = stream

    if (audioUnlocked.value) {
      el.play().catch(console.warn)
    }
  }
}

const setAudioRef = (el: Element | ComponentPublicInstance | null): void => {
  audioRef.value = el instanceof HTMLAudioElement ? el : null
  if (audioRef.value) setAudioStream()
}

onMounted(() => {
  window.addEventListener('click', unlockAudio, { once: true })
})

onBeforeUnmount(() => {
  unmounted = true
  window.removeEventListener('click', unlockAudio)

  source?.disconnect()
  highPass?.disconnect()
  lowPass?.disconnect()
  gain?.disconnect()

  void audioContext.close().catch(() => {})
})

watch([() => props.participant.muted, () => props.participant.stream], setAudioStream)
</script>

<template>
  <audio :ref="setAudioRef" autoplay playsinline muted />
</template>
