<script setup lang="ts">
import { nextTick, ref, type Component, type ComponentPublicInstance } from 'vue'
import type { MeetingConfigSectionKey } from '@/stores/meetingConfig'

interface SectionItem {
  key: MeetingConfigSectionKey
  label: string
  icon: Component
}

const props = defineProps<{
  sections: SectionItem[]
}>()

const active = defineModel<MeetingConfigSectionKey>({ required: true })

const tabRefs = ref<HTMLButtonElement[]>([])

function setTabRef(index: number, el: Element | ComponentPublicInstance | null) {
  if (el instanceof HTMLButtonElement) tabRefs.value[index] = el
}

function onTabKeydown(event: KeyboardEvent, index: number) {
  const last = props.sections.length - 1
  let next = index

  if (event.key === 'ArrowDown' || event.key === 'ArrowRight') {
    next = index === last ? 0 : index + 1
  } else if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') {
    next = index === 0 ? last : index - 1
  } else if (event.key === 'Home') {
    next = 0
  } else if (event.key === 'End') {
    next = last
  } else {
    return
  }

  const target = props.sections[next]
  if (!target) return

  event.preventDefault()
  active.value = target.key
  void nextTick(() => tabRefs.value[next]?.focus())
}
</script>

<template>
  <nav
    aria-label="Configuration sections"
    class="shrink-0 overflow-x-auto border-b border-gray-200 md:w-56 md:overflow-x-hidden md:overflow-y-auto md:border-r md:border-b-0 chat-scroll"
  >
    <ul class="flex gap-1 p-2 md:flex-col" role="tablist" aria-label="Configuration sections">
      <li v-for="(section, index) in sections" :key="section.key" class="shrink-0">
        <button
          :id="`config-tab-${section.key}`"
          :ref="(el) => setTabRef(index, el)"
          type="button"
          role="tab"
          :aria-selected="active === section.key"
          :aria-controls="`config-panel-${section.key}`"
          :tabindex="active === section.key ? 0 : -1"
          class="flex w-full items-center gap-x-2.5 whitespace-nowrap rounded-lg px-3 py-2 text-left text-sm font-medium transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
          :class="
            active === section.key
              ? 'bg-primary/10 text-primary'
              : 'text-gray-600 hover:bg-gray-100 hover:text-gray-900'
          "
          @click="active = section.key"
          @keydown="onTabKeydown($event, index)"
        >
          <component :is="section.icon" class="size-4 shrink-0" aria-hidden="true" />
          <span>{{ section.label }}</span>
        </button>
      </li>
    </ul>
  </nav>
</template>

<style scoped>
.chat-scroll {
  scrollbar-width: thin;
  scrollbar-color: rgb(209 213 219) transparent;
}

.chat-scroll::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

.chat-scroll::-webkit-scrollbar-thumb {
  border-radius: 9999px;
  background-color: rgb(209 213 219);
}

.chat-scroll::-webkit-scrollbar-track {
  background: transparent;
}
</style>
