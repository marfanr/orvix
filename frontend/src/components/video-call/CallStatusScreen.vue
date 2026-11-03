<script setup lang="ts">
import { computed, ref } from 'vue'
import { CheckIcon, CopyIcon } from 'lucide-vue-next'
import HeaderBrand from '@/components/HeaderBrand.vue'

const props = defineProps<{
  status: 'denied' | 'waiting' | 'not-started' | 'not-found'
  roomId: string
  roomOwner?: string
}>()

const META: Record<
  typeof props.status,
  { title: string; description: string; pending: boolean; tone: 'alert' | 'status' }
> = {
  denied: {
    title: 'Request denied',
    description: 'The host denied your request to join this meeting.',
    pending: false,
    tone: 'alert',
  },
  waiting: {
    title: 'Asking to join…',
    description: "We'll let you in as soon as the host admits you.",
    pending: true,
    tone: 'status',
  },
  'not-started': {
    title: "The meeting hasn't started",
    description: 'Come back when the host is ready to begin.',
    pending: true,
    tone: 'status',
  },
  'not-found': {
    title: 'Meeting room not found',
    description: 'Check the room code or link, then try again.',
    pending: false,
    tone: 'alert',
  },
}

const meta = computed(() => META[props.status])
const roomLabel = computed(() =>
  props.roomOwner ? `${props.roomOwner}'s meeting room` : 'Meeting room',
)
const roomLink = computed(() =>
  typeof window !== 'undefined' ? `${window.location.origin}/meet/${props.roomId}` : props.roomId,
)

const copied = ref(false)

async function copyLink() {
  try {
    await navigator.clipboard.writeText(roomLink.value)
    copied.value = true
    setTimeout(() => (copied.value = false), 1600)
  } catch {
    copied.value = false
  }
}
</script>

<template>
  <div
    class="relative flex min-h-dvh w-full flex-col bg-gray-50"
    :role="meta.tone"
    :aria-live="meta.tone === 'alert' ? 'assertive' : 'polite'"
  >
    <header class="flex items-center gap-2 px-6 py-5 sm:px-10" aria-hidden="true">
      <HeaderBrand/>
    </header>

    <main class="flex flex-1 flex-col items-center justify-center px-6 pb-16 text-center">
      <div
        v-if="meta.pending"
        class="mb-6 flex items-center gap-1 motion-safe:animate-pulse"
        aria-hidden="true"
      >
        <span class="size-1.5 rounded-full bg-primary" />
        <span class="size-1.5 rounded-full bg-primary [animation-delay:400ms]" />
        <span class="size-1.5 rounded-full bg-primary [animation-delay:800ms]" />
      </div>

      <h1 class="font-heading text-2xl font-medium tracking-tight text-gray-900 sm:text-3xl">
        {{ meta.title }}
      </h1>
      <p class="mt-2 max-w-sm text-sm text-gray-500">{{ meta.description }}</p>

      <div class="mt-10 flex items-center gap-x-3" aria-hidden="false">
        <span class="h-px w-10 bg-gray-200" aria-hidden="true"></span>
        <p class="font-mono text-xs text-gray-600 tabular-nums" :title="roomId">{{ roomId }}</p>
        <span class="h-px w-10 bg-gray-200" aria-hidden="true"></span>
      </div>
      <p class="mt-1.5 text-[11px] uppercase tracking-widest text-gray-400">
        {{ roomLabel }}
      </p>

      <div class="mt-10 flex items-center gap-x-2">
        <button
          type="button"
          @click="copyLink"
          class="flex min-h-10 cursor-pointer items-center gap-x-2 rounded-lg border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 transition-colors duration-150 hover:border-gray-400 hover:bg-gray-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
        >
          <CheckIcon v-if="copied" :size="16" class="text-success" aria-hidden="true" />
          <CopyIcon v-else :size="16" aria-hidden="true" />
          {{ copied ? 'Copied' : 'Copy link' }}
        </button>
        <RouterLink
          to="/"
          class="flex min-h-10 cursor-pointer items-center gap-x-2 rounded-lg bg-primary px-4 py-2 text-sm font-medium text-white transition-colors duration-150 hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
        >
          Back to home
        </RouterLink>
      </div>
    </main>
  </div>
</template>
