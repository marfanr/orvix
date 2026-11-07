// SPDX-FileCopyrightText: 2026 Mohammad Arfan
//
// SPDX-License-Identifier: AGPL-3.0

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { ViewLayoutMode, type Participant } from '@/types/video-call'
import VideoTile from './VideoTile.vue'
import { useCallStore } from '@/stores/call.ts'
import { storeToRefs } from 'pinia'

const store = useCallStore()
const { viewLayout } = storeToRefs(store)

const props = defineProps<{
  participants: Participant[]
}>()

const DESKTOP_WIDTH = 768
const DESKTOP_RATIO = 16 / 9
const MOBILE_MIN_RATIO = 3 / 4
const MOBILE_MAX_RATIO = 16 / 9

const gridRef = ref<HTMLElement | null>(null)
const size = ref({ width: 0, height: 0 })

let resizeObserver: ResizeObserver | null = null

const isDesktop = computed(() => size.value.width >= DESKTOP_WIDTH)
const gap = computed(() => (size.value.width >= 640 ? 8 : 4))

const columns = computed(() => {
  const n = props.participants.length

  if (isDesktop.value) {
    if (n <= 1) return 1
    if (n <= 4) return 2
    if (n <= 6) return 3
    return 4
  }

  if (n <= 2) return 1
  if (n <= 8) return 2
  return 3
})

const layout = computed(() => {
  const n = props.participants.length
  const { width, height } = size.value
  const g = gap.value

  if (n === 0 || width <= 0 || height <= 0) return { width: 0, height: 0, rowWidth: 0 }

  const cols = Math.min(columns.value, n)
  const rows = Math.ceil(n / cols)

  const minRatio = isDesktop.value ? DESKTOP_RATIO : MOBILE_MIN_RATIO
  const maxRatio = isDesktop.value ? DESKTOP_RATIO : MOBILE_MAX_RATIO

  const cellW = (width - g * (cols - 1)) / cols
  const cellH = (height - g * (rows - 1)) / rows

  let w = cellW
  let h = cellH
  const ratio = cellW / cellH

  if (ratio > maxRatio) w = cellH * maxRatio
  else if (ratio < minRatio) h = cellW / minRatio

  w = Math.floor(w)
  h = Math.floor(h)

  return { width: w, height: h, rowWidth: w * cols + g * (cols - 1) }
})

// speaker mode: peserta yang di-pin tampil di stage, sisanya di sidebar
const pinnedId = ref<Participant['id'] | null>(null)

const stageParticipant = computed(
  () => props.participants.find((p) => p.id === pinnedId.value) ?? props.participants[0],
)

const sideParticipants = computed(() =>
  props.participants.filter((p) => p.id !== stageParticipant.value?.id),
)

function pin(id: Participant['id']) {
  pinnedId.value = id
}

onMounted(() => {
  const el = gridRef.value
  if (!el) return

  size.value = { width: el.clientWidth, height: el.clientHeight }

  resizeObserver = new ResizeObserver((entries) => {
    const rect = entries[0]?.contentRect
    if (rect) size.value = { width: rect.width, height: rect.height }
  })
  resizeObserver.observe(el)
})

onBeforeUnmount(() => {
  resizeObserver?.disconnect()
})

const isShowGrid = computed(() => viewLayout.value === ViewLayoutMode.Grid)
const isShowSpeaker = computed(() => viewLayout.value === ViewLayoutMode.Speaker)
</script>

<template>
  <main ref="gridRef" class="w-full h-full min-h-0 overflow-x-hidden"
    :class="isShowSpeaker ? 'overflow-y-hidden p-2' : 'overflow-y-auto p-2 sm:p-4'">
    <!-- grid mode -->
    <div v-if="isShowGrid" class="flex h-full w-full items-center justify-center">
      <div v-if="layout.width > 0" class="flex flex-wrap justify-center"
        :style="{ width: `${layout.rowWidth}px`, gap: `${gap}px` }">
        <div v-for="p in participants" :key="p.id" class="shrink-0"
          :style="{ width: `${layout.width}px`, height: `${layout.height}px` }">
          <VideoTile :participant="p" tile-class="w-full h-full rounded-sm" />
        </div>
      </div>
    </div>

    <!-- speaker mode: filmstrip di atas, stage besar di bawah -->
    <div v-else-if="isShowSpeaker" class="flex h-full w-full min-h-0 flex-col gap-2">
      <!-- filmstrip member lain -->
      <div v-if="sideParticipants.length > 0"
        class="flex h-20 sm:h-24 shrink-0 flex-row gap-2 overflow-x-auto overflow-y-hidden [scrollbar-width:thin]">
        <button v-for="p in sideParticipants" :key="p.id" type="button"
          class="relative h-full aspect-video shrink-0 cursor-pointer overflow-hidden rounded-xl bg-stone-900 outline-none
                 ring-2 ring-transparent transition duration-150
                 hover:ring-stone-400 focus-visible:ring-stone-500" @click="pin(p.id)">
          <VideoTile :participant="p" tile-class="w-full h-full" />
        </button>
      </div>

      <!-- stage -->
      <div class="relative flex grow min-h-0 min-w-0 items-stretch justify-center">
        <div class="relative aspect-video max-w-full overflow-hidden rounded-xl bg-stone-900">
          <VideoTile v-if="stageParticipant" :key="stageParticipant.id" :participant="stageParticipant"
            tile-class="w-full h-full" />
        </div>
      </div>
    </div>
  </main>
</template>

<style lang="css" module></style>
