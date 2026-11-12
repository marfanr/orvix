<!--
SPDX-FileCopyrightText: 2026 Mohammad Arfan

SPDX-License-Identifier: AGPL-3.0
-->

<script setup lang="ts">
import type { Participant } from '@/types/video-call'
import { nextTick, ref, watch, type ComponentPublicInstance } from 'vue'

const props = defineProps<{
  participant: Participant
}>()

const audioRef = ref<HTMLAudioElement | null>(null)
// const audioContext = new AudioContext()
// // let meteredStream: MediaStream | null = null
// // let stopLevelMeter: (() => void) | undefined

// // const createLevelMeter = () => {
// //   console.log('create levl emetr')
// //   if (!meteredStream) return
// //   const source = audioContext.createMediaStreamSource(meteredStream)
// //   const analyzer = audioContext.createAnalyser()

// //   analyzer.fftSize = 512
// //   source.connect(analyzer)

// //   const buf = new Float32Array(analyzer.fftSize)
// //   const timer = setInterval(() => {
// //     analyzer.getFloatTimeDomainData(buf)

// //     let sum = 0
// //     for (let i = 0; i < buf.length; i++) sum += buf[i]! * buf[i]!
// //     const level = Math.sqrt(sum / buf.length)
// //     console.log('level', level)
// //   }, 100)
// //   return () => {
// //     clearInterval(timer)
// //     source.disconnect()
// //     analyzer.disconnect()
// //   }
// // }

async function setAudioStream(): Promise<void> {
  await nextTick()
  const { stream, muted, id } = props.participant
  if (!audioRef.value || !stream) {
    // if (stopLevelMeter) stopLevelMeter()
    return
  }

  if (id !== 0) {
    audioRef.value.muted = muted
    const el = audioRef.value
    if (el.srcObject !== stream) el.srcObject = stream
    el.play().catch((err) => console.warn(`${el.tagName.toLowerCase()} play error:`, err))
  }

  // if (meteredStream !== stream) {
  //   if (stopLevelMeter) stopLevelMeter()
  //   meteredStream = stream
  //   stopLevelMeter = createLevelMeter()
  // }
}

const setAudioRef = (el: Element | ComponentPublicInstance | null): void => {
  audioRef.value = el instanceof HTMLAudioElement ? el : null
  if (audioRef.value) setAudioStream()
}

watch([() => props.participant.muted, () => props.participant.stream], setAudioStream)
</script>

<template>
  <audio :ref="setAudioRef" autoplay playsinline />
</template>
