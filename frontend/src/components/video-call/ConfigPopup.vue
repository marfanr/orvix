<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch, type Component } from 'vue'
import { createFocusTrap, type FocusTrap } from 'focus-trap'
import { HardDriveIcon, MicIcon, SlidersHorizontalIcon, VideoIcon, WifiIcon } from 'lucide-vue-next'
import PopUpComponent from '@/components/ui/PopUpComponent.vue'
import ConfigSidebar from '@/components/video-call/config/ConfigSidebar.vue'
import NetworkConfigSection from '@/components/video-call/config/sections/NetworkConfigSection.vue'
import VideoConfigSection from '@/components/video-call/config/sections/VideoConfigSection.vue'
import AudioConfigSection from '@/components/video-call/config/sections/AudioConfigSection.vue'
import RecordingConfigSection from '@/components/video-call/config/sections/RecordingConfigSection.vue'
import AdvancedConfigSection from '@/components/video-call/config/sections/AdvancedConfigSection.vue'

import { useConfigPanel } from '@/composables/useConfigPanel.tsx'
import { PopUpWidth } from '@/types/video-call.ts'
import  { type MeetingConfigSectionKey, useMeetingConfigStore } from '@/stores/meetingConfig.ts'

const { isOpen, close } = useConfigPanel()
const { save } = useMeetingConfigStore()

const popupRef = ref<{ contentEl: HTMLElement | null } | null>(null)
let trap: FocusTrap | null = null

const sections: {
  key: MeetingConfigSectionKey
  label: string
  icon: Component
  component: Component
}[] = [
  {
    key: 'network',
    label: 'Network & bandwidth',
    icon: WifiIcon,
    component: NetworkConfigSection,
  },
  { key: 'video', label: 'Video', icon: VideoIcon, component: VideoConfigSection },
  { key: 'audio', label: 'Audio', icon: MicIcon, component: AudioConfigSection },
  {
    key: 'recording',
    label: 'Recording',
    icon: HardDriveIcon,
    component: RecordingConfigSection,
  },
  {
    key: 'advanced',
    label: 'Advanced',
    icon: SlidersHorizontalIcon,
    component: AdvancedConfigSection,
  },
]

const activeSection = ref<MeetingConfigSectionKey>('network')
const currentSection = computed(
  () => sections.find((section) => section.key === activeSection.value) ?? sections[0],
)

watch(
  isOpen,
  async (open) => {
    if (!open) {
      trap?.deactivate()
      trap = null
      return
    }

    await nextTick()

    const contentEl = popupRef.value?.contentEl
    if (!contentEl) return

    trap = createFocusTrap(contentEl, { returnFocusOnDeactivate: true })
    trap.activate()
  },
  { immediate: true },
)

function onDocKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') close()
}

onMounted(() => {
  document.addEventListener('keydown', onDocKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', onDocKeydown)
  trap?.deactivate()
  trap = null
})
</script>

<template>
  <PopUpComponent ref="popupRef" :is-open="isOpen" @close="close()" :width="PopUpWidth.LARGE">
    <template #title>Configuration</template>

    <div class="flex min-h-0 flex-1 flex-col">
      <div class="flex min-h-0 flex-1 flex-col md:flex-row">
        <ConfigSidebar v-model="activeSection" :sections="sections" />

        <section
          :id="`config-panel-${activeSection}`"
          role="tabpanel"
          :aria-labelledby="`config-tab-${activeSection}`"
          tabindex="0"
          class="min-w-0 flex-1 overflow-y-auto p-4 focus-visible:outline-none sm:p-6 chat-scroll"
        >
          <div class="mx-auto max-w-2xl">
            <component v-if="currentSection" :is="currentSection.component" />
          </div>
        </section>
      </div>

      <footer
        class="flex shrink-0 items-center justify-end gap-x-2 border-t border-gray-200 px-4 py-3 sm:px-6"
      >
        <button
          type="button"
          class="rounded-lg px-3 py-2 text-sm font-medium text-gray-600 transition hover:bg-gray-100 hover:text-gray-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
          @click="close()"
        >
          Cancel
        </button>
        <button
          type="button"
          class="rounded-lg bg-primary px-4 py-2 text-sm font-medium text-white transition hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
          @click="save(); close()"
        >
          Save changes
        </button>
      </footer>
    </div>
  </PopUpComponent>
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
