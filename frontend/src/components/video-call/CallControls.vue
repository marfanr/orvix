<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, type Component } from 'vue'
import {
  CameraIcon,
  CameraOffIcon,
  CircleDotIcon,
  MessageSquarePlusIcon,
  MicIcon,
  MicOffIcon,
  MoreHorizontalIcon,
  PhoneOffIcon,
  ScreenShareIcon,
  UsersIcon,
  XIcon,
} from 'lucide-vue-next'
import ReactionPicker from './ReactionPicker.vue'
import type { DataControlType } from '@/composables/useWebRTC.ts'
import { useMembersPanel } from '@/composables/useMembersPanel.ts'
import { useCallStore } from '@/stores/call.ts'
import { storeToRefs } from 'pinia'
import { BotIcon } from '@lucide/vue'

defineProps<{
  cameraOff: boolean
  muted: boolean
}>()

const emit = defineEmits<{
  'toggle-camera': []
  'toggle-mic': []
  'send-control': [msg: DataControlType]
  hangup: []
  'select-reaction': [emoji: string]
}>()

const membersPanel = useMembersPanel()
const store = useCallStore()
const { isChatOpen } = storeToRefs(store)
const { toggleChat } = store

const actionClass =
  'grid size-11 shrink-0 place-items-center rounded-full transition cursor-pointer focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary touch-manipulation active:scale-95'

const moreOpen = ref(false)
const moreButtonRef = ref<HTMLButtonElement | null>(null)
const panelRef = ref<HTMLElement | null>(null)
const popoverStyle = ref<Record<string, string>>({})
const isDesktop = ref(false)

let mql: MediaQueryList | null = null

const MENU_WIDTH = 240
const MENU_GAP = 8
const EDGE = 8

function updatePosition() {
  const el = moreButtonRef.value
  if (!el) return

  const rect = el.getBoundingClientRect()
  const left = Math.min(
    Math.max(rect.right - MENU_WIDTH, EDGE),
    window.innerWidth - MENU_WIDTH - EDGE,
  )

  popoverStyle.value = {
    left: `${left}px`,
    bottom: `${window.innerHeight - rect.top + MENU_GAP}px`,
  }
}

function onMqlChange(event: MediaQueryListEvent) {
  isDesktop.value = event.matches
  if (moreOpen.value) updatePosition()
}

async function openMore() {
  moreOpen.value = true
  updatePosition()

  await nextTick()
  updatePosition()

  const first = panelRef.value?.querySelector<HTMLButtonElement>('button:not([disabled])')
  first?.focus()
}

function closeMore(returnFocus = true) {
  if (!moreOpen.value) return

  moreOpen.value = false

  if (returnFocus) {
    void nextTick(() => moreButtonRef.value?.focus())
  }
}

function toggleMore() {
  if (moreOpen.value) closeMore()
  else void openMore()
}

function onRecordClick() {
  emit('send-control', JSON.stringify({ type: 'record' }))
}

function onParticipantsClick() {
  membersPanel.toggle()
}

function onAiInviteClick() {
  console.log("invite ai")
  emit('send-control', JSON.stringify({ type: 'invite_ai' }))
}

interface MenuItem {
  key: string
  label: string
  description: string
  icon: Component
  disabled?: boolean
  hideOnDesktop?: boolean
  onSelect?: () => void
}

const menuItems = computed<MenuItem[]>(() => [
  {
    key: 'screen-share',
    label: 'Share screen',
    description: 'Coming soon',
    icon: ScreenShareIcon,
    disabled: true,
  },
  {
    key: 'record',
    label: 'Record meeting',
    description: 'Start a server recording',
    icon: CircleDotIcon,
    onSelect: onRecordClick,
    disabled: true
  },
  {
    key: 'participants',
    label: 'Participants',
    description: 'See who is in the call',
    icon: UsersIcon,
    hideOnDesktop: true,
    onSelect: onParticipantsClick,
  },
  {
    key: 'ai',
    label: 'invite AI',
    description: 'invite AI into this room',
    icon: BotIcon,
    onSelect: onAiInviteClick
  }
])

function runMenuItem(item: MenuItem) {
  if (item.disabled || !item.onSelect) return

  closeMore(false)
  item.onSelect()
}

function onReactionSelect(emoji: string) {
  emit('select-reaction', emoji)
  closeMore()
}

function onDocPointerDown(event: PointerEvent) {
  if (!moreOpen.value) return

  const target = event.target as Element | null
  if (!target) return

  if (moreButtonRef.value?.contains(target)) return
  if (panelRef.value?.contains(target)) return
  if (target.closest('[data-reaction-picker]')) return

  closeMore(false)
}

function onDocKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape' && moreOpen.value) closeMore()
}

onMounted(() => {
  mql = window.matchMedia('(min-width: 640px)')
  isDesktop.value = mql.matches
  mql.addEventListener('change', onMqlChange)
  document.addEventListener('pointerdown', onDocPointerDown)
  document.addEventListener('keydown', onDocKeydown)
  window.addEventListener('resize', updatePosition)
})

onUnmounted(() => {
  mql?.removeEventListener('change', onMqlChange)
  document.removeEventListener('pointerdown', onDocPointerDown)
  document.removeEventListener('keydown', onDocKeydown)
  window.removeEventListener('resize', updatePosition)
})
</script>

<template>
  <div class="w-full px-3 pb-[max(0.75rem,env(safe-area-inset-bottom))] sm:px-6 sm:pb-4">
    <div
      class="mx-auto flex w-full max-w-md items-center justify-center gap-x-2 rounded-2xl border border-gray-200 bg-white/95 p-2 shadow-lg backdrop-blur sm:max-w-none sm:gap-x-3 sm:border-0 sm:bg-transparent sm:p-0 sm:shadow-none sm:backdrop-blur-none"
    >
      <button
        type="button"
        :aria-label="muted ? 'Unmute microphone' : 'Mute microphone'"
        :title="muted ? 'Unmute microphone' : 'Mute microphone'"
        :aria-pressed="muted"
        :class="[
          actionClass,
          muted
            ? 'bg-primary/10 text-primary hover:bg-primary/15'
            : 'bg-gray-100 text-gray-800 hover:bg-gray-200',
        ]"
        @click="emit('toggle-mic')"
      >
        <MicOffIcon v-if="muted" class="size-5" aria-hidden="true" />
        <MicIcon v-else class="size-5" aria-hidden="true" />
      </button>

      <button
        type="button"
        :aria-label="cameraOff ? 'Turn camera on' : 'Turn camera off'"
        :title="cameraOff ? 'Turn camera on' : 'Turn camera off'"
        :aria-pressed="cameraOff"
        :class="[
          actionClass,
          cameraOff
            ? 'bg-primary/10 text-primary hover:bg-primary/15'
            : 'bg-gray-100 text-gray-800 hover:bg-gray-200',
        ]"
        @click="emit('toggle-camera')"
      >
        <CameraOffIcon v-if="cameraOff" class="size-5" aria-hidden="true" />
        <CameraIcon v-else class="size-5" aria-hidden="true" />
      </button>

      <div class="hidden sm:block">
        <button
          type="button"
          aria-label="Participants"
          title="Participants"
          :class="[actionClass, 'bg-gray-100 text-gray-700 hover:bg-gray-200']"
          @click="onParticipantsClick"
        >
          <UsersIcon class="size-5" aria-hidden="true" />
        </button>
      </div>

      <button
        type="button"
        :aria-label="isChatOpen ? 'Close chat' : 'Open chat'"
        :title="isChatOpen ? 'Close chat' : 'Open chat'"
        :aria-pressed="isChatOpen"
        :class="[
          actionClass,
          isChatOpen
            ? 'bg-primary/10 text-primary hover:bg-primary/15'
            : 'bg-gray-100 text-gray-700 hover:bg-gray-200',
        ]"
        @click="toggleChat"
      >
        <MessageSquarePlusIcon class="size-5" aria-hidden="true" />
      </button>

      <div class="hidden sm:block">
        <ReactionPicker @select="onReactionSelect" />
      </div>

      <button
        type="button"
        aria-label="Leave call"
        title="Leave call"
        class="grid size-11 shrink-0 place-items-center rounded-full bg-primary text-white transition hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary touch-manipulation active:scale-95"
        @click="emit('hangup')"
      >
        <PhoneOffIcon class="size-5" aria-hidden="true" />
      </button>

      <button
        ref="moreButtonRef"
        type="button"
        aria-label="More controls"
        title="More controls"
        aria-haspopup="true"
        :aria-expanded="moreOpen"
        :class="[
          actionClass,
          moreOpen ? 'bg-primary/10 text-primary' : 'bg-gray-100 text-gray-700 hover:bg-gray-200',
        ]"
        @click="toggleMore"
      >
        <MoreHorizontalIcon class="size-5" aria-hidden="true" />
      </button>
    </div>

    <Teleport to="body">
      <div
        v-if="moreOpen"
        class="fixed bottom-0 right-0 left-0 z-50 sm:inset-auto"
        :style="isDesktop ? popoverStyle : undefined"
      >
        <div
          class="absolute inset-0 bg-black/40 sm:hidden"
          aria-hidden="true"
          @click="closeMore(false)"
        ></div>

        <div
          ref="panelRef"
          class="relative mt-auto flex max-h-[75dvh] w-full flex-col overflow-hidden rounded-t-2xl bg-white shadow-2xl sm:mt-0 sm:max-h-none sm:w-60 sm:rounded-xl sm:border sm:border-gray-200 sm:shadow-xl"
          aria-label="More controls"
        >
          <div
            class="flex shrink-0 items-center justify-between border-b border-gray-200 px-4 py-3 sm:hidden"
          >
            <span class="text-sm font-semibold text-gray-900">More controls</span>
            <button
              type="button"
              aria-label="Close more controls"
              class="grid size-9 place-items-center rounded-full text-gray-500 transition hover:bg-gray-100 hover:text-gray-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
              @click="closeMore()"
            >
              <XIcon class="size-5" aria-hidden="true" />
            </button>
          </div>

          <div
            class="min-h-0 flex-1 overflow-y-auto p-1.5 pb-[max(0.5rem,env(safe-area-inset-bottom))] no-scrollbar sm:pb-1.5"
          >
            <ul class="flex flex-col gap-0.5">
              <li
                v-for="item in menuItems"
                :key="item.key"
                :class="{ 'sm:hidden': item.hideOnDesktop }"
              >
                <button
                  type="button"
                  :disabled="item.disabled"
                  :title="item.disabled ? `${item.label} (coming soon)` : item.label"
                  class="flex w-full items-center gap-x-3 rounded-lg px-3 py-2.5 text-left text-sm font-medium text-gray-700 transition hover:bg-gray-100 focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-primary disabled:cursor-not-allowed disabled:text-gray-400 disabled:hover:bg-transparent"
                  @click="runMenuItem(item)"
                >
                  <component :is="item.icon" class="size-5 shrink-0" aria-hidden="true" />
                  <span class="min-w-0 flex-1 truncate">{{ item.label }}</span>
                  <span
                    v-if="item.disabled"
                    class="shrink-0 rounded-full bg-gray-100 px-2 py-0.5 text-[10px] font-medium text-gray-500"
                  >
                    Soon
                  </span>
                </button>
              </li>

              <li class="mt-0.5 border-t border-gray-100 pt-1 sm:hidden">
                <ReactionPicker variant="menu" @select="onReactionSelect" />
              </li>
            </ul>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>
