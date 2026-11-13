<script setup lang="ts">
import {
  CalendarClockIcon,
  CalendarDaysIcon,
  CalendarPlusIcon,
  CheckIcon,
  CopyIcon,
  DoorOpenIcon,
  HistoryIcon,
  LogInIcon,
  LogOutIcon,
  PlusIcon,
  RotateCcwIcon,
  UserIcon,
  VideoIcon,
} from 'lucide-vue-next'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import HeaderBrand from '@/components/HeaderBrand.vue'
import NewMeetingPopup from '@/components/home/NewMeetingPopup.vue'
import { api } from '@/lib/axios'
import { listRecentMeetings, removeRecentMeeting, type RecentMeeting } from '@/lib/recentMeetings'
import {
  listScheduledMeetings,
  removeScheduledMeeting,
  type ScheduledMeeting,
} from '@/lib/scheduledMeetings'
import { userIdFromToken } from '@/lib/token'
import { useAuthStore } from '@/stores/auth'
import type { NewMeetingMode } from '@/types/video-call'

const router = useRouter()
const auth = useAuthStore()

const userId = computed(() => userIdFromToken(auth.accessToken))

const personalAlias = computed(() => (userId.value ? `me-${userId.value}` : ''))
const personalLink = computed(() =>
  personalAlias.value ? `${window.location.origin}/meet/${personalAlias.value}` : '',
)

const accountOpen = ref(false)
const toggleAccountMenu = () => (accountOpen.value = !accountOpen.value)
const closeAccountMenu = () => (accountOpen.value = false)

const handleLogout = async () => {
  closeAccountMenu()
  await auth.logout()
  await router.push('/signin')
}

const mode = ref<NewMeetingMode>('now')
const showNewMeeting = ref(false)
const openNewMeeting = () => {
  showNewMeeting.value = true
  mode.value = 'now'
}
const openScheduledMeeting = () => {
  showNewMeeting.value = true
  mode.value = 'scheduled'
}
const closeNewMeeting = () => {
  showNewMeeting.value = false
}
const switchNewMeetingMode = (m: string) => {
  mode.value = m as NewMeetingMode
}

const upcoming = ref<ScheduledMeeting[]>([])
const upcomingError = ref('')
const loadUpcoming = () => {
  upcoming.value = listScheduledMeetings(userId.value)
}

const sampleMeetings = computed<ScheduledMeeting[]>(() => {
  const base = new Date()
  const at = (addDays: number, hour: number) => {
    const date = new Date(base)
    date.setDate(date.getDate() + addDays)
    date.setHours(hour, 0, 0, 0)
    return date.toISOString()
  }

  return [
    {
      id: 'sample-standup',
      alias: 'sample-daily-standup',
      title: 'Daily standup',
      startAt: at(0, base.getHours() + 1),
      durationMin: 30,
      timezone: 'Local time',
      sample: true,
    },
    {
      id: 'sample-design',
      alias: 'sample-design-review',
      title: 'Design review',
      startAt: at(1, 10),
      durationMin: 45,
      timezone: 'Local time',
      sample: true,
    },
    {
      id: 'sample-sync',
      alias: 'sample-weekly-sync',
      title: 'Weekly sync',
      startAt: at(2, 14),
      durationMin: 60,
      timezone: 'Local time',
      sample: true,
    },
  ]
})

const shownMeetings = computed(() =>
  upcoming.value.length ? upcoming.value : sampleMeetings.value,
)
const usingSamples = computed(() => upcoming.value.length === 0)

const joinScheduled = async (meeting: ScheduledMeeting) => {
  upcomingError.value = ''

  if (meeting.sample) {
    try {
      await api.post('/room', {
        title: meeting.title,
        alias: meeting.alias,
        owner_id: userId.value,
        options: JSON.stringify({ waiting_room: false }),
      })
    } catch (err) {
      const status = (err as { response?: { status?: number } }).response?.status
      if (status !== 409) {
        upcomingError.value = 'Could not open this meeting. Check your connection and try again.'
        return
      }
    }
  }

  await router.push({ name: 'video-call', params: { id: meeting.alias } })
}

const removeScheduled = (meeting: ScheduledMeeting) => {
  removeScheduledMeeting(userId.value, meeting.id)
  loadUpcoming()
}

const recent = ref<RecentMeeting[]>([])
const recentError = ref('')
const loadRecent = () => {
  recent.value = listRecentMeetings(userId.value)
}

const sampleRecent = computed<RecentMeeting[]>(() => {
  const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString()
  return [
    {
      alias: 'sample-sprint-planning',
      title: 'Sprint planning',
      lastOpenedAt: ago(90),
      sample: true,
    },
    {
      alias: 'sample-marketing-sync',
      title: 'Marketing sync',
      lastOpenedAt: ago(60 * 26),
      sample: true,
    },
    {
      alias: 'sample-retro',
      title: 'Retro',
      lastOpenedAt: ago(60 * 24 * 4),
      sample: true,
    },
  ]
})

const shownRecent = computed(() => (recent.value.length ? recent.value : sampleRecent.value))
const usingRecentSamples = computed(() => recent.value.length === 0)

const rejoinRecent = async (meeting: RecentMeeting) => {
  recentError.value = ''

  if (meeting.sample) {
    try {
      await api.post('/room', {
        title: meeting.title,
        alias: meeting.alias,
        owner_id: userId.value,
        options: JSON.stringify({ waiting_room: false }),
      })
    } catch (err) {
      const status = (err as { response?: { status?: number } }).response?.status
      if (status !== 409) {
        recentError.value = 'Could not open this meeting. Check your connection and try again.'
        return
      }
    }
  }

  await router.push({ name: 'video-call', params: { id: meeting.alias } })
}

const removeRecent = (meeting: RecentMeeting) => {
  removeRecentMeeting(userId.value, meeting.alias)
  loadRecent()
}

const relativeTime = (iso: string) => {
  const minutes = Math.round((Date.now() - Date.parse(iso)) / 60_000)
  if (minutes < 1) return 'Just now'
  if (minutes < 60) return `${minutes} min ago`

  const hours = Math.round(minutes / 60)
  if (hours < 24) return `${hours} hr ago`

  const days = Math.round(hours / 24)
  if (days === 1) return 'Yesterday'
  if (days < 7) return `${days} days ago`

  return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
}

const startOfDay = (date: Date) =>
  new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime()

const dayLabel = (iso: string) => {
  const date = new Date(iso)
  const offset = Math.round((startOfDay(date) - startOfDay(new Date())) / 86_400_000)
  if (offset === 0) return 'Today'
  if (offset === 1) return 'Tomorrow'
  return date.toLocaleDateString(undefined, { weekday: 'short', month: 'short', day: 'numeric' })
}

const timeRange = (meeting: ScheduledMeeting) => {
  const start = new Date(meeting.startAt)
  const end = new Date(start.getTime() + meeting.durationMin * 60_000)
  const format = new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit' })
  return `${format.format(start)}–${format.format(end)}`
}

const roomInput = ref('')
const joinError = ref('')

const extractRoomId = (value: string) => {
  const trimmed = value.trim()
  if (!trimmed) return ''

  try {
    const url = new URL(trimmed, window.location.origin)
    const match = url.pathname.match(/\/meet\/([^/?#]+)/)
    if (match?.[1]) return decodeURIComponent(match[1])
  } catch {
    return ''
  }

  return trimmed
    .replace(/^\/+/, '')
    .replace(/^meet\//, '')
    .split(/[/?#]/)[0]
}

const joinRoom = async () => {
  const id = extractRoomId(roomInput.value)
  if (!id) {
    joinError.value = 'Enter a meeting link or room code.'
    return
  }

  joinError.value = ''
  await router.push({ name: 'video-call', params: { id } })
}

const personalError = ref('')
const personalReady = ref(false)
const openingRoom = ref(false)
const copied = ref(false)
let copiedTimer: ReturnType<typeof setTimeout> | undefined

const openPersonalRoom = async () => {
  if (openingRoom.value) return
  openingRoom.value = true
  personalError.value = ''

  const ready = await ensurePersonalRoom()
  openingRoom.value = false
  if (ready) {
    await router.push({ name: 'video-call', params: { id: personalAlias.value } })
  }
}

async function ensurePersonalRoom() {
  if (personalReady.value) return true
  if (!personalAlias.value || !userId.value) {
    personalError.value = 'Your account details are unavailable. Sign in again and retry.'
    return false
  }

  try {
    await api.post('/room', {
      title: 'Personal room',
      alias: personalAlias.value,
      owner_id: userId.value,
      options: JSON.stringify({ waiting_room: false }),
    })
    personalReady.value = true
    return true
  } catch (err) {
    const status = (err as { response?: { status?: number } }).response?.status
    if (status === 409) {
      personalReady.value = true
      return true
    }
    personalError.value = 'Could not reach your room. Check your connection and try again.'
    return false
  }
}

const copyPersonalLink = async () => {
  personalError.value = ''
  if (!(await ensurePersonalRoom())) return

  try {
    await navigator.clipboard.writeText(personalLink.value)
    copied.value = true
    clearTimeout(copiedTimer)
    copiedTimer = setTimeout(() => (copied.value = false), 1800)
  } catch {
    personalError.value = 'Clipboard access is unavailable in this browser.'
  }
}

const onKeydown = (e: KeyboardEvent) => {
  if (e.key === 'Escape' && accountOpen.value) closeAccountMenu()
}

onMounted(() => {
  document.addEventListener('keydown', onKeydown)
  loadUpcoming()
  loadRecent()
})
onUnmounted(() => {
  document.removeEventListener('keydown', onKeydown)
  clearTimeout(copiedTimer)
})

const primaryBtnCls =
  'inline-flex min-h-11 items-center justify-center gap-x-2 rounded-lg bg-primary px-5 text-sm font-semibold text-white transition hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary'
const secondaryBtnCls =
  'inline-flex min-h-11 items-center justify-center rounded-lg border border-stone-300 bg-white px-5 text-sm font-semibold text-gray-800 transition hover:bg-stone-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary'
const smallSecondaryBtnCls =
  'inline-flex min-h-9 items-center justify-center rounded-md border border-stone-300 bg-white px-3 text-sm font-medium text-gray-800 transition hover:bg-stone-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary disabled:opacity-60'
</script>

<template>
  <div class="flex min-h-dvh flex-col bg-primary-soft/30" @click="closeAccountMenu">
    <header class="border-b border-stone-200 bg-white">
      <div
        class="mx-auto flex h-16 w-full max-w-5xl items-center justify-between gap-4 px-4 sm:px-6"
      >
        <RouterLink
          to="/"
          class="flex items-center gap-x-2.5 rounded-lg focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-primary"
          aria-label="Orvix home"
        >
          <HeaderBrand />
        </RouterLink>

        <div class="relative" @click.stop>
          <button
            type="button"
            :aria-expanded="accountOpen"
            aria-haspopup="true"
            aria-label="Account menu"
            class="flex min-h-10 items-center gap-x-2 rounded-lg px-2 text-sm font-medium text-gray-700 transition hover:bg-stone-100 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
            @click="toggleAccountMenu"
          >
            <span
              class="grid size-8 place-items-center rounded-full bg-stone-100 text-gray-600"
              aria-hidden="true"
            >
              <UserIcon class="size-4" />
            </span>
            <span class="hidden sm:inline">Account</span>
          </button>

          <div
            v-if="accountOpen"
            class="absolute right-0 z-50 mt-2 w-52 overflow-hidden rounded-lg border border-stone-200 bg-white p-1.5 shadow-lg"
          >
            <p class="px-3 py-2 text-xs text-gray-500">Signed in to Orvix</p>
            <button
              type="button"
              class="flex w-full items-center gap-x-2.5 rounded-md px-3 py-2 text-left text-sm text-danger transition hover:bg-danger/5 focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-primary"
              @click="handleLogout"
            >
              <LogOutIcon class="size-4" aria-hidden="true" />
              Log out
            </button>
          </div>
        </div>
      </div>
    </header>

    <main id="main" class="mx-auto w-full max-w-5xl flex-1 px-4 py-10 sm:px-6 sm:py-14">
      <div
        class="grid gap-10"
        :class="personalAlias ? 'lg:grid-cols-[minmax(0,1fr)_21rem] lg:gap-14' : ''"
      >
        <div>
          <h1 class="font-heading text-3xl font-semibold tracking-tight text-gray-900 sm:text-4xl">
            Start or join a meeting
          </h1>
          <p class="mt-3 max-w-md text-base leading-relaxed text-gray-600">
            Create a room and share the link, or enter a room code to join a call already in
            progress.
          </p>

          <button type="button" :class="[primaryBtnCls, 'mt-7']" @click="openNewMeeting">
            <PlusIcon class="size-4" aria-hidden="true" />
            New meeting
          </button>

          <section class="mt-10 border-t border-stone-300/70 pt-8" aria-labelledby="join-title">
            <h2
              id="join-title"
              class="flex items-center gap-x-2 text-sm font-semibold text-gray-900"
            >
              <LogInIcon class="size-4 text-gray-400" aria-hidden="true" />
              Join a meeting
            </h2>
            <p class="mt-1 text-sm text-gray-600">Paste the invite link or type the room code.</p>

            <form class="mt-4 flex flex-col gap-2 sm:flex-row" @submit.prevent="joinRoom">
              <label for="room-code" class="sr-only">Meeting link or room code</label>
              <input
                id="room-code"
                v-model="roomInput"
                type="text"
                name="room-code"
                autocomplete="off"
                spellcheck="false"
                placeholder="Paste a link or room code"
                class="h-11 min-w-0 flex-1 rounded-lg border border-stone-300 bg-white px-3 text-sm text-gray-900 placeholder:text-gray-500 focus:border-primary focus:ring-2 focus:ring-primary/20 focus:outline-none"
                :aria-invalid="!!joinError"
                :aria-describedby="joinError ? 'room-code-error' : undefined"
              />
              <button type="submit" :class="[secondaryBtnCls, 'gap-x-2']">
                <LogInIcon class="size-4" aria-hidden="true" />
                Join
              </button>
            </form>

            <p v-if="joinError" id="room-code-error" role="alert" class="mt-2 text-sm text-danger">
              {{ joinError }}
            </p>
          </section>

          <section class="mt-10 border-t border-stone-300/70 pt-8" aria-labelledby="recent-title">
            <div class="flex items-baseline justify-between gap-x-3">
              <h2
                id="recent-title"
                class="flex items-center gap-x-2 text-sm font-semibold text-gray-900"
              >
                <HistoryIcon class="size-4 text-gray-400" aria-hidden="true" />
                Recent meetings
              </h2>
              <span v-if="usingRecentSamples" class="text-xs text-gray-500">Sample</span>
            </div>
            <p class="mt-1 text-xs text-gray-500">
              {{
                usingRecentSamples
                  ? 'Sample history. Rooms you join will replace these.'
                  : 'Rooms you joined on this device.'
              }}
            </p>

            <ul class="mt-4 divide-y divide-stone-200 border-y border-stone-200">
              <li
                v-for="meeting in shownRecent"
                :key="meeting.alias"
                class="flex flex-wrap items-center gap-x-3 gap-y-2 py-3"
              >
                <span
                  class="grid size-9 shrink-0 place-items-center rounded-lg bg-stone-100 text-gray-500"
                  aria-hidden="true"
                >
                  <HistoryIcon class="size-4" />
                </span>
                <div class="min-w-0 flex-1">
                  <h3 class="truncate text-sm font-semibold text-gray-900">
                    {{ meeting.title }}
                  </h3>
                  <p class="mt-0.5 text-xs text-gray-500">
                    {{ relativeTime(meeting.lastOpenedAt) }}
                  </p>
                </div>
                <div class="ml-auto flex shrink-0 items-center gap-x-1">
                  <button
                    type="button"
                    :class="[smallSecondaryBtnCls, 'gap-x-1.5']"
                    :aria-label="`Rejoin ${meeting.title}`"
                    @click="rejoinRecent(meeting)"
                  >
                    <RotateCcwIcon class="size-3.5" aria-hidden="true" />
                    Rejoin
                  </button>
                  <button
                    v-if="!meeting.sample"
                    type="button"
                    class="inline-flex min-h-9 items-center rounded-md px-2 text-sm font-medium text-gray-500 transition hover:text-danger focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
                    :aria-label="`Remove ${meeting.title} from history`"
                    @click="removeRecent(meeting)"
                  >
                    Remove
                  </button>
                </div>
              </li>
            </ul>

            <p v-if="recentError" role="alert" class="mt-2 text-sm text-danger">
              {{ recentError }}
            </p>
          </section>
        </div>

        <aside
          v-if="personalAlias"
          class="border-t border-stone-300/70 pt-8 lg:border-t-0 lg:border-l lg:pt-1 lg:pl-10"
        >
          <section aria-labelledby="upcoming-title">
            <div class="flex items-baseline justify-between gap-x-3">
              <h2
                id="upcoming-title"
                class="flex items-center gap-x-2 text-sm font-semibold text-gray-900"
              >
                <CalendarClockIcon class="size-4 text-gray-400" aria-hidden="true" />
                Upcoming meetings
              </h2>
              <button
                type="button"
                class="inline-flex items-center gap-x-1.5 rounded-md text-sm font-medium text-primary transition hover:underline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
                @click="openScheduledMeeting"
              >
                <CalendarPlusIcon class="size-4" aria-hidden="true" />
                Schedule
              </button>
            </div>
            <p class="mt-1 text-xs leading-relaxed text-gray-500">
              {{
                usingSamples
                  ? 'Sample meetings. Schedule your own to replace them.'
                  : 'Scheduled meetings are stored on this device.'
              }}
            </p>

            <ul class="mt-4 divide-y divide-stone-200 border-y border-stone-200">
              <li
                v-for="meeting in shownMeetings"
                :key="meeting.id"
                class="flex items-start gap-x-3 py-3"
              >
                <span
                  class="mt-0.5 grid size-9 shrink-0 place-items-center rounded-lg bg-stone-100 text-gray-500"
                  aria-hidden="true"
                >
                  <CalendarDaysIcon class="size-4" />
                </span>
                <div class="min-w-0 flex-1">
                  <p class="text-[11px] font-medium text-gray-500">
                    {{ dayLabel(meeting.startAt) }} · {{ timeRange(meeting) }} ·
                    {{ meeting.durationMin }} min
                  </p>
                  <div class="mt-1 flex items-start justify-between gap-x-3">
                    <h3 class="min-w-0 truncate text-sm font-semibold text-gray-900">
                      {{ meeting.title }}
                    </h3>
                    <button
                      type="button"
                      class="-mt-0.5 shrink-0 rounded-md px-2 py-0.5 text-xs font-medium text-primary transition hover:bg-primary/5 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
                      :aria-label="`Join ${meeting.title}`"
                      @click="joinScheduled(meeting)"
                    >
                      Join
                    </button>
                  </div>
                  <button
                    v-if="!meeting.sample"
                    type="button"
                    class="mt-1 rounded-md text-xs text-gray-500 transition hover:text-danger focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
                    :aria-label="`Remove ${meeting.title} from the list`"
                    @click="removeScheduled(meeting)"
                  >
                    Remove
                  </button>
                </div>
              </li>
            </ul>

            <p v-if="upcomingError" role="alert" class="mt-2 text-xs text-danger">
              {{ upcomingError }}
            </p>
          </section>

          <section class="mt-8 border-t border-stone-300/70 pt-8" aria-labelledby="room-title">
            <h2
              id="room-title"
              class="flex items-center gap-x-2 text-sm font-semibold text-gray-900"
            >
              <VideoIcon class="size-4 text-gray-400" aria-hidden="true" />
              Your room
            </h2>
            <p class="mt-1 text-sm leading-relaxed text-gray-600">
              A room that stays yours. Share the link so people can drop in without booking a time.
            </p>

            <p class="mt-5 text-xs text-gray-500">Room code</p>
            <p class="mt-1 truncate font-mono text-xs text-gray-700" :title="personalLink">
              {{ personalAlias }}
            </p>

            <div class="mt-3 flex flex-wrap items-center gap-x-4 gap-y-2">
              <button
                type="button"
                class="inline-flex min-h-9 items-center gap-x-1.5 rounded-md px-3 text-sm font-medium text-primary transition hover:bg-primary/5 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
                @click="copyPersonalLink"
              >
                <component
                  :is="copied ? CheckIcon : CopyIcon"
                  class="size-4"
                  :class="copied ? 'text-success' : ''"
                  aria-hidden="true"
                />
                <span aria-live="polite">{{ copied ? 'Link copied' : 'Copy link' }}</span>
              </button>

              <button
                type="button"
                :class="[smallSecondaryBtnCls, 'gap-x-1.5']"
                :disabled="openingRoom"
                :aria-busy="openingRoom"
                @click="openPersonalRoom"
              >
                <DoorOpenIcon class="size-3.5" aria-hidden="true" />
                {{ openingRoom ? 'Opening…' : 'Open room' }}
              </button>
            </div>

            <p v-if="personalError" role="alert" class="mt-3 text-sm text-danger">
              {{ personalError }}
            </p>
          </section>
        </aside>
      </div>
    </main>

    <footer class="mx-auto w-full max-w-5xl px-4 pb-8 sm:px-6">
      <p class="border-t border-stone-300/70 pt-5 text-xs text-gray-500">
        Orvix is self-hosted. Your meeting media stays on your own server.
      </p>
    </footer>

    <NewMeetingPopup
      :is_open="showNewMeeting"
      :mode="mode"
      @close="closeNewMeeting"
      @scheduled="loadUpcoming"
      @switch-new-meeting-mode="switchNewMeetingMode"
    />
  </div>
</template>
