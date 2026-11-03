<script setup lang="ts">
import { api } from '@/lib/axios'
import {
  ArrowRightIcon,
  BellIcon,
  CalendarDaysIcon,
  CheckCircle2Icon,
  ClockIcon,
  LogOutIcon,
  PlusIcon,
  SettingsIcon,
  SparklesIcon,
  UserIcon,
  UsersIcon,
  VideoIcon,
  XIcon,
  WebcamIcon
} from 'lucide-vue-next'
import { nextTick, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { nanoid } from 'nanoid'
import { createFocusTrap, type FocusTrap } from 'focus-trap'
import { useAuthStore } from '@/stores/auth'
import InputElement from '@/components/ui/InputElement.vue'
import SelectElement from '@/components/ui/SelectElement.vue'
import ToggleElement from '@/components/ui/ToggleElement.vue'

/* ---------- navigation ---------- */
const navigationItems = ref([
  { name: 'Meet', icon: 'video', active: true },
  { name: 'Contacts', icon: 'users', active: false },
])

const setActive = (name: string) => {
  navigationItems.value = navigationItems.value.map((item) => ({
    ...item,
    active: item.name === name,
  }))
}

/* ---------- profile menu ---------- */
const showProfileMenu = ref(false)
const toggleProfileMenu = () => (showProfileMenu.value = !showProfileMenu.value)
const closeProfileMenu = () => (showProfileMenu.value = false)

const todayLabel = new Date().toLocaleDateString('en-US', {
  weekday: 'long',
  month: 'long',
  day: 'numeric',
})

const upcomingMeetings = ref([
  { id: 1, title: 'Team Standup', time: '09:00', date: 'Today', participants: 8 },
  { id: 2, title: 'Project Review', time: '14:00', date: 'Today', participants: 12 },
  { id: 3, title: 'Client Call', time: '16:30', date: 'Tomorrow', participants: 5 },
])

const recentMeetings = ref([
  { id: 4, title: 'Sprint Planning', time: '10:00', date: 'Yesterday', participants: 10 },
  { id: 5, title: 'Design Review', time: '15:00', date: '2 days ago', participants: 6 },
])

type Mode = 'now' | 'scheduled'

const modes: { key: Mode; label: string }[] = [
  { key: 'now', label: 'Start now' },
  { key: 'scheduled', label: 'Schedule' },
]

const today = () => new Date().toLocaleDateString('en-CA') // yyyy-mm-dd, local time
const genPasscode = () => Math.random().toString(36).slice(2, 8)
const personalId = nanoid()

const showNewMeeting = ref(false)
const mode = ref<Mode>('now')

const nowForm = reactive({
  topic: "Arfan's meeting",
  meetingId: 'auto' as 'auto' | 'personal' | 'custom',
  hostVideo: true,
  participantVideo: false,
  muteOnEntry: true,
  waitingRoom: true,
  requestToJoin: true,
})

const nowToggles = [
  {
    key: 'hostVideo',
    label: 'Host video on',
    description: 'Start the call with your camera enabled.',
  },
  {
    key: 'participantVideo',
    label: 'Participant video on',
    description: 'Let participants join with their camera enabled.',
  },
  {
    key: 'muteOnEntry',
    label: 'Mute participants on entry',
    description: 'Everyone joins muted to keep the room quiet.',
  },
  {
    key: 'waitingRoom',
    label: 'Waiting room',
    description: 'Hold guests until the host admits them.',
  },
  {
    key: 'requestToJoin',
    label: 'Request to join',
    description: 'Ask the host for permission before entering.',
  },
] as const

const scheduledForm = reactive({
  topic: '',
  date: today(),
  time: '09:00',
  duration: '45 minutes',
  timezone: 'WIB (UTC+7)',
  passcode: genPasscode(),
  waitingRoom: true,
  requestToJoin: true,
})

const durations = ['15 minutes', '30 minutes', '45 minutes', '1 hour', '2 hours']
const timezones = ['WIB (UTC+7)', 'WITA (UTC+8)', 'WIT (UTC+9)']

const durationOptions = durations.map((duration) => ({ value: duration, label: duration }))
const timezoneOptions = timezones.map((timezone) => ({ value: timezone, label: timezone }))

const submitting = ref(false)
const submitError = ref('')
const submitSuccess = ref('')

const openNewMeeting = () => {
  mode.value = 'now'
  scheduledForm.date = today()
  scheduledForm.passcode = genPasscode()
  submitError.value = ''
  submitSuccess.value = ''
  showNewMeeting.value = true
}

const openScheduledMeeting = () => {
  openNewMeeting()
  mode.value = 'scheduled'
}

const closeNewMeeting = () => (showNewMeeting.value = false)

const currentUserId = (): string => {
  const token = useAuthStore().accessToken
  if (!token) return ''
  try {
    const payload = JSON.parse(atob(token.split('.')[1] ?? ''))
    return String(payload.sub ?? '')
  } catch {
    return ''
  }
}

const submitMeeting = async () => {
  if (submitting.value) return
  submitError.value = ''
  submitSuccess.value = ''
  submitting.value = true

  try {
    if (mode.value === 'now') {
      const options = JSON.stringify({
        host_video: nowForm.hostVideo,
        participant_video: nowForm.participantVideo,
        mute_on_entry: nowForm.muteOnEntry,
        waiting_room: nowForm.waitingRoom,
      })

      await api.post('/room', {
        alias: nowForm.topic.trim() || 'Untitled meeting',
        owner_id: currentUserId(),
        passcode: genPasscode(),
        options,
      })

      submitSuccess.value = 'Meeting started. Sharing your room link…'
      closeNewMeeting()
    } else {
      const f = scheduledForm
      upcomingMeetings.value.push({
        id: Date.now(),
        title: f.topic.trim() || 'Untitled meeting',
        time: f.time,
        date: f.date === today() ? 'Today' : f.date,
        participants: 1,
      })
      submitSuccess.value = 'Meeting scheduled.'
      closeNewMeeting()
    }
  } catch (err) {
    const axiosErr = err as { response?: { data?: { error?: string } } }
    submitError.value =
      axiosErr.response?.data?.error || 'Could not create the meeting. Please try again.'
  } finally {
    submitting.value = false
  }
}

/* ---------- modal focus trap ---------- */
const dialogEl = ref<HTMLElement | null>(null)
let trap: FocusTrap | null = null

watch(showNewMeeting, async (open) => {
  if (open) {
    await nextTick()
    if (dialogEl.value) {
      trap = createFocusTrap(dialogEl.value, { returnFocusOnDeactivate: true })
      trap.activate()
    }
  } else {
    trap?.deactivate()
    trap = null
  }
})

const onKeydown = (e: KeyboardEvent) => {
  if (e.key !== 'Escape') return
  if (showNewMeeting.value) closeNewMeeting()
  else if (showProfileMenu.value) closeProfileMenu()
}
onMounted(() => document.addEventListener('keydown', onKeydown))
onUnmounted(() => {
  document.removeEventListener('keydown', onKeydown)
  trap?.deactivate()
})

/* shared classes */
const radioCls =
  'flex cursor-pointer items-start gap-x-3 rounded-xl border border-stone-200 p-3.5 transition has-checked:border-primary has-checked:bg-primary/5 hover:border-stone-300'
const menuItemCls =
  'flex w-full items-center gap-x-2.5 rounded-lg px-3 py-2 text-left text-sm text-gray-700 transition hover:bg-stone-100 focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-primary'
</script>

<template>
  <div class="min-h-screen bg-primary-soft/30" @click="closeProfileMenu">
    <header class="sticky top-0 z-40 border-b border-stone-200/80 bg-white/80 backdrop-blur-md">
      <div class="mx-auto flex h-16 max-w-6xl items-center gap-2 px-4 sm:gap-3 sm:px-6 lg:px-8">
        <div class="flex items-center gap-x-2.5">
          <span
            class="grid size-9 place-items-center rounded-xl bg-primary text-white shadow-sm"
            aria-hidden="true"
          >
            <WebcamIcon class="size-5" />
          </span>
          <span
            class="hidden font-heading text-lg font-semibold tracking-tight text-gray-900 sm:block"
          >
            Orvix
          </span>
        </div>

        <nav
          class="ml-1 flex items-center gap-1 rounded-full bg-stone-100/80 p-1 sm:ml-3"
          aria-label="Primary"
        >
          <button
            v-for="item in navigationItems"
            :key="item.name"
            type="button"
            @click="setActive(item.name)"
            :aria-current="item.active ? 'page' : undefined"
            :class="[
              'rounded-full px-3 py-1.5 text-xs font-medium transition-all sm:px-4 sm:text-sm',
              item.active
                ? 'bg-white text-gray-900 shadow-sm'
                : 'text-gray-500 hover:text-gray-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary',
            ]"
          >
            {{ item.name }}
          </button>
        </nav>

        <div class="ml-auto flex items-center gap-1 sm:gap-2">
          <button
            type="button"
            aria-label="Notifications"
            class="relative grid size-10 place-items-center rounded-full text-gray-600 transition hover:bg-stone-100 hover:text-gray-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
          >
            <BellIcon class="size-5" aria-hidden="true" />
            <span
              class="absolute top-2.5 right-2.5 size-2 rounded-full bg-danger ring-2 ring-white"
              aria-hidden="true"
            ></span>
          </button>

          <div class="relative" @click.stop>
            <button
              type="button"
              @click="toggleProfileMenu"
              aria-haspopup="true"
              :aria-expanded="showProfileMenu"
              aria-label="Account menu"
              class="grid size-9 place-items-center rounded-full bg-primary text-sm font-semibold text-white transition hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
            >
              <span aria-hidden="true">A</span>
            </button>

            <div
              v-if="showProfileMenu"
              class="absolute right-0 z-50 mt-2 w-56 overflow-hidden rounded-xl border border-stone-200 bg-white p-1.5 shadow-xl"
            >
              <div class="px-3 py-2">
                <p class="text-sm font-semibold text-gray-900">Account</p>
                <p class="text-xs text-gray-500">Signed in to VidCall</p>
              </div>
              <div class="my-1 h-px bg-stone-100" aria-hidden="true"></div>
              <button type="button" :class="menuItemCls" @click="closeProfileMenu">
                <UserIcon class="size-4" aria-hidden="true" />
                Profile
              </button>
              <button type="button" :class="menuItemCls" @click="closeProfileMenu">
                <SettingsIcon class="size-4" aria-hidden="true" />
                Settings
              </button>
              <div class="my-1 h-px bg-stone-100" aria-hidden="true"></div>
              <button
                type="button"
                :class="[menuItemCls, 'text-danger hover:bg-danger/5']"
                @click="closeProfileMenu"
              >
                <LogOutIcon class="size-4" aria-hidden="true" />
                Logout
              </button>
            </div>
          </div>
        </div>
      </div>
    </header>

    <main id="main" class="mx-auto max-w-6xl px-4 py-6 sm:px-6 sm:py-10 lg:px-8">
      <div
        v-if="submitSuccess"
        role="status"
        class="mb-6 flex items-start gap-x-3 rounded-xl border border-success/20 bg-white px-4 py-3 text-sm text-gray-700 shadow-sm"
      >
        <CheckCircle2Icon class="mt-0.5 size-5 shrink-0 text-success" aria-hidden="true" />
        <p class="min-w-0 flex-1">{{ submitSuccess }}</p>
        <button
          type="button"
          aria-label="Dismiss message"
          class="grid size-6 shrink-0 place-items-center rounded-full text-gray-400 transition hover:bg-stone-100 hover:text-gray-700 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
          @click="submitSuccess = ''"
        >
          <XIcon class="size-4" aria-hidden="true" />
        </button>
      </div>

      <section
        class="relative overflow-hidden rounded-3xl bg-primary px-5 py-8 text-white shadow-xl shadow-primary/20 sm:px-8 sm:py-10 lg:px-10"
      >
        <div
          class="pointer-events-none absolute -top-24 -right-16 size-64 rounded-full bg-white/10 blur-2xl"
          aria-hidden="true"
        ></div>
        <div
          class="pointer-events-none absolute -bottom-28 left-1/3 size-56 rounded-full bg-primary-hover/30 blur-2xl"
          aria-hidden="true"
        ></div>

        <div class="relative flex flex-col gap-8 lg:flex-row lg:items-end lg:justify-between">
          <div class="max-w-xl">
            <p
              class="flex items-center gap-x-2 text-xs font-medium tracking-wide text-white/70 uppercase"
            >
              <SparklesIcon class="size-4" aria-hidden="true" />
              {{ todayLabel }}
            </p>
            <h1 class="mt-3 font-heading text-3xl font-semibold tracking-tight sm:text-4xl">
              Ready when you are.
            </h1>
            <p class="mt-3 text-sm leading-relaxed text-white/80 sm:text-base">
              Start an instant call or plan one for later. Your room, your link, and everyone
              together in a few clicks.
            </p>

            <div class="mt-6 flex flex-wrap items-center gap-2.5">
              <button
                type="button"
                class="inline-flex items-center gap-x-2 rounded-xl bg-white px-4 py-2.5 text-sm font-semibold text-primary shadow-sm transition hover:bg-white/90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
                @click="openNewMeeting"
              >
                <PlusIcon class="size-4" aria-hidden="true" />
                New meeting
              </button>
              <button
                type="button"
                class="inline-flex items-center gap-x-2 rounded-xl bg-white/10 px-4 py-2.5 text-sm font-medium text-white ring-1 ring-white/25 transition hover:bg-white/20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
                @click="openScheduledMeeting"
              >
                <CalendarDaysIcon class="size-4" aria-hidden="true" />
                Schedule
              </button>
            </div>
          </div>

          <div
            class="hidden w-64 shrink-0 rounded-2xl bg-white/10 p-4 ring-1 ring-white/20 backdrop-blur-sm lg:block"
          >
            <p class="text-[11px] font-medium tracking-wide text-white/60 uppercase">Next up</p>
            <template v-if="upcomingMeetings[0]">
              <p class="mt-2 truncate text-sm font-semibold">
                {{ upcomingMeetings[0].title }}
              </p>
              <p class="mt-1 flex items-center gap-x-1.5 text-xs text-white/70">
                <ClockIcon class="size-3.5" aria-hidden="true" />
                {{ upcomingMeetings[0].date }} · {{ upcomingMeetings[0].time }}
              </p>
              <p class="mt-3 flex items-center gap-x-1.5 text-xs text-white/70">
                <UsersIcon class="size-3.5" aria-hidden="true" />
                {{ upcomingMeetings[0].participants }} participants
              </p>
            </template>
            <p v-else class="mt-2 text-sm text-white/70">Nothing scheduled yet.</p>
          </div>
        </div>
      </section>

      <div class="mt-8 grid gap-8 lg:grid-cols-[minmax(0,1fr)_18rem] lg:items-start">
        <div class="min-w-0 space-y-10">
          <section id="upcoming" aria-labelledby="upcoming-title">
            <div class="flex flex-wrap items-end justify-between gap-3">
              <div>
                <h2 id="upcoming-title" class="font-heading text-xl font-semibold text-gray-900">
                  Upcoming meetings
                </h2>
                <p class="mt-1 text-sm text-gray-500">Your next calls, all in one place.</p>
              </div>
              <span
                class="rounded-full bg-white px-2.5 py-1 text-xs font-medium text-gray-500 ring-1 ring-stone-200"
              >
                {{ upcomingMeetings.length }} scheduled
              </span>
            </div>

            <div
              v-if="upcomingMeetings.length"
              class="mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-3"
            >
              <article
                v-for="meeting in upcomingMeetings"
                :key="meeting.id"
                class="group flex flex-col rounded-2xl border border-stone-200 bg-white p-4 shadow-sm transition hover:-translate-y-0.5 hover:border-primary/30 hover:shadow-md"
              >
                <div class="flex items-center justify-between gap-2">
                  <span
                    class="inline-flex items-center gap-x-1.5 rounded-full bg-primary/10 px-2.5 py-1 text-[11px] font-semibold text-primary"
                  >
                    <CalendarDaysIcon class="size-3.5" aria-hidden="true" />
                    {{ meeting.date }}
                  </span>
                  <span class="flex items-center gap-x-1 text-xs text-gray-400">
                    <UsersIcon class="size-3.5" aria-hidden="true" />
                    {{ meeting.participants }}
                  </span>
                </div>

                <h3 class="mt-4 truncate text-base font-semibold text-gray-900">
                  {{ meeting.title }}
                </h3>
                <p class="mt-1 flex items-center gap-x-1.5 text-sm text-gray-500">
                  <ClockIcon class="size-4" aria-hidden="true" />
                  {{ meeting.time }}
                </p>

                <span
                  class="mt-4 inline-flex items-center gap-x-1 text-xs font-medium text-primary opacity-0 transition group-hover:opacity-100"
                >
                  View details
                  <ArrowRightIcon class="size-3.5" aria-hidden="true" />
                </span>
              </article>
            </div>

            <div
              v-else
              class="mt-4 flex flex-col items-center justify-center rounded-2xl border border-dashed border-stone-300 bg-white/60 px-6 py-10 text-center"
            >
              <span class="grid size-11 place-items-center rounded-full bg-primary/10 text-primary">
                <CalendarDaysIcon class="size-5" aria-hidden="true" />
              </span>
              <p class="mt-3 text-sm font-medium text-gray-800">No meetings scheduled</p>
              <p class="mt-1 text-sm text-gray-500">Plan one and it will show up here.</p>
              <button
                type="button"
                class="mt-4 inline-flex items-center gap-x-2 rounded-xl bg-primary px-4 py-2.5 text-sm font-medium text-white transition hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
                @click="openScheduledMeeting"
              >
                <PlusIcon class="size-4" aria-hidden="true" />
                Schedule a meeting
              </button>
            </div>
          </section>

          <section aria-labelledby="recent-title">
            <div>
              <h2 id="recent-title" class="font-heading text-xl font-semibold text-gray-900">
                Recent
              </h2>
              <p class="mt-1 text-sm text-gray-500">Jump back into a room you used before.</p>
            </div>

            <ul
              class="mt-4 divide-y divide-stone-100 overflow-hidden rounded-2xl border border-stone-200 bg-white shadow-sm"
            >
              <li v-for="meeting in recentMeetings" :key="meeting.id">
                <div
                  class="flex items-center gap-x-3 px-4 py-3.5 transition hover:bg-stone-50 sm:px-5"
                >
                  <span
                    class="grid size-11 shrink-0 place-items-center rounded-xl bg-primary/10 text-primary"
                    aria-hidden="true"
                  >
                    <VideoIcon class="size-5" />
                  </span>

                  <div class="min-w-0 flex-1">
                    <p class="truncate text-sm font-medium text-gray-900">{{ meeting.title }}</p>
                    <p class="mt-0.5 flex flex-wrap items-center gap-x-1.5 text-xs text-gray-500">
                      <span>{{ meeting.date }}</span>
                      <span aria-hidden="true">·</span>
                      <span>{{ meeting.time }}</span>
                      <span aria-hidden="true">·</span>
                      <span>{{ meeting.participants }} participants</span>
                    </p>
                  </div>

                  <span class="shrink-0 text-gray-300" aria-hidden="true">
                    <ChevronRightIcon class="size-4" />
                  </span>
                </div>
              </li>

              <li v-if="!recentMeetings.length" class="px-5 py-8 text-center text-sm text-gray-500">
                No recent meetings yet.
              </li>
            </ul>
          </section>
        </div>

        <aside class="space-y-4 lg:sticky lg:top-24">
          <div class="rounded-2xl border border-stone-200 bg-white p-5 shadow-sm">
            <div class="flex items-center gap-x-2.5">
              <span class="grid size-9 place-items-center rounded-lg bg-primary/10 text-primary">
                <VideoIcon class="size-4" aria-hidden="true" />
              </span>
              <h2 class="text-sm font-semibold text-gray-900">Your personal room</h2>
            </div>

            <p class="mt-3 text-xs leading-relaxed text-gray-500">
              Share this ID when someone wants to join you directly.
            </p>
            <p
              class="mt-2 overflow-hidden rounded-lg bg-stone-50 px-3 py-2 font-mono text-xs text-gray-700 break-all ring-1 ring-stone-100"
            >
              {{ personalId }}
            </p>

            <button
              type="button"
              class="mt-4 inline-flex w-full items-center justify-center gap-x-2 rounded-xl bg-primary px-4 py-2.5 text-sm font-medium text-white transition hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
              @click="openNewMeeting"
            >
              <PlusIcon class="size-4" aria-hidden="true" />
              Start a meeting
            </button>
          </div>

          <div class="rounded-2xl border border-stone-200 bg-white p-5 shadow-sm">
            <div class="flex items-center gap-x-2.5">
              <span class="grid size-9 place-items-center rounded-lg bg-stone-100 text-gray-600">
                <CalendarDaysIcon class="size-4" aria-hidden="true" />
              </span>
              <h2 class="text-sm font-semibold text-gray-900">Plan ahead</h2>
            </div>
            <p class="mt-3 text-xs leading-relaxed text-gray-500">
              Pick a time, set a duration, and share the invite when you are ready.
            </p>
            <button
              type="button"
              class="mt-4 inline-flex w-full items-center justify-center gap-x-2 rounded-xl border border-stone-200 bg-white px-4 py-2.5 text-sm font-medium text-gray-700 transition hover:border-primary/40 hover:text-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
              @click="openScheduledMeeting"
            >
              <CalendarDaysIcon class="size-4" aria-hidden="true" />
              Schedule a meeting
            </button>
          </div>
        </aside>
      </div>
    </main>

    <!-- New Meeting modal -->
    <Teleport to="body">
      <Transition
        enter-active-class="transition duration-150 ease-out"
        enter-from-class="opacity-0"
        enter-to-class="opacity-100"
        leave-active-class="transition duration-100 ease-in"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
      >
        <div
          v-if="showNewMeeting"
          class="fixed inset-0 z-99 flex items-center justify-center bg-gray-950/40 px-4 py-6 backdrop-blur-[2px] sm:px-6"
          @click.self="closeNewMeeting"
        >
          <div
            ref="dialogEl"
            role="dialog"
            aria-modal="true"
            aria-labelledby="nm-title"
            class="flex max-h-[calc(100dvh-3rem)] w-full max-w-2xl flex-col overflow-hidden rounded-2xl border border-stone-200 bg-white shadow-2xl"
          >
            <div class="flex items-start justify-between gap-x-4 px-5 pt-5 pb-4 sm:px-6">
              <div class="flex items-start gap-x-3">
                <span
                  class="grid size-10 shrink-0 place-items-center rounded-xl bg-primary/10 text-primary"
                  aria-hidden="true"
                >
                  <VideoIcon class="size-5" />
                </span>
                <div>
                  <h3 id="nm-title" class="font-heading text-lg font-semibold text-gray-900">
                    New meeting
                  </h3>
                  <p class="mt-0.5 text-xs text-gray-500">
                    Start instantly or schedule it for later.
                  </p>
                </div>
              </div>

              <button
                type="button"
                aria-label="Close"
                @click="closeNewMeeting"
                class="grid size-8 shrink-0 place-items-center rounded-full text-gray-500 transition hover:bg-stone-100 hover:text-gray-900 focus-visible:outline-2 focus-visible:outline-primary"
              >
                <XIcon class="size-5" aria-hidden="true" />
              </button>
            </div>

            <div class="px-5 sm:px-6">
              <div role="tablist" class="grid grid-cols-2 gap-1 rounded-xl bg-stone-100 p-1">
                <button
                  v-for="m in modes"
                  :key="m.key"
                  role="tab"
                  type="button"
                  :aria-selected="mode === m.key"
                  @click="mode = m.key"
                  class="rounded-lg px-4 py-2 text-sm font-medium transition focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-primary"
                  :class="
                    mode === m.key
                      ? 'bg-white text-gray-900 shadow-sm'
                      : 'text-gray-500 hover:text-gray-800'
                  "
                >
                  {{ m.label }}
                </button>
              </div>
            </div>

            <div class="flex-1 overflow-y-auto px-5 py-5 sm:px-6">
              <form
                v-if="mode === 'now'"
                class="flex flex-col gap-y-5"
                @submit.prevent="submitMeeting"
              >
                <InputElement
                  id="now-topic"
                  v-model="nowForm.topic"
                  label="Topic"
                  :hide-label="false"
                  placeholder="Team standup"
                />

                <fieldset class="flex flex-col gap-y-2">
                  <legend class="mb-1 text-xs font-semibold tracking-wide text-gray-500 uppercase">
                    Meeting ID
                  </legend>

                  <label :class="radioCls">
                    <input
                      v-model="nowForm.meetingId"
                      type="radio"
                      value="auto"
                      class="mt-0.5 size-4 accent-primary"
                    />
                    <span class="min-w-0">
                      <span class="block text-sm font-medium text-gray-800">
                        Generate automatically
                      </span>
                      <span class="mt-0.5 block text-xs text-gray-500">
                        A fresh room ID for this meeting.
                      </span>
                    </span>
                  </label>

                  <label :class="radioCls">
                    <input
                      v-model="nowForm.meetingId"
                      type="radio"
                      value="personal"
                      class="mt-0.5 size-4 accent-primary"
                    />
                    <span class="min-w-0 flex-1">
                      <span class="block text-sm font-medium text-gray-800">
                        Personal meeting ID
                      </span>
                      <span class="mt-0.5 block text-xs text-gray-500">
                        Reuse the same room for every call.
                      </span>
                    </span>
                    <span class="shrink-0 font-mono text-[11px] text-gray-500">
                      {{ personalId }}
                    </span>
                  </label>

                  <label :class="radioCls">
                    <input
                      v-model="nowForm.meetingId"
                      type="radio"
                      value="custom"
                      class="mt-0.5 size-4 accent-primary"
                    />
                    <span class="min-w-0">
                      <span class="block text-sm font-medium text-gray-800">Custom</span>
                      <span class="mt-0.5 block text-xs text-gray-500">
                        Choose an ID yourself.
                      </span>
                    </span>
                  </label>
                </fieldset>

                <div class="divide-y divide-stone-100 rounded-xl border border-stone-200">
                  <ToggleElement
                    v-for="t in nowToggles"
                    :key="t.key"
                    v-model="nowForm[t.key]"
                    :label="t.label"
                    :description="t.description"
                  />
                </div>
              </form>

              <form v-else class="flex flex-col gap-y-5" @submit.prevent="submitMeeting">
                <InputElement
                  id="sc-topic"
                  v-model="scheduledForm.topic"
                  label="Topic"
                  :hide-label="false"
                  placeholder="Weekly sync"
                />

                <div class="grid gap-4 sm:grid-cols-2">
                  <InputElement
                    id="sc-date"
                    v-model="scheduledForm.date"
                    type="date"
                    label="Date"
                    :hide-label="false"
                    :min="today()"
                  />
                  <InputElement
                    id="sc-time"
                    v-model="scheduledForm.time"
                    type="time"
                    label="Start time"
                    :hide-label="false"
                  />
                </div>

                <div class="grid gap-4 sm:grid-cols-2">
                  <SelectElement
                    id="sc-dur"
                    v-model="scheduledForm.duration"
                    label="Duration"
                    :options="durationOptions"
                  />
                  <SelectElement
                    id="sc-tz"
                    v-model="scheduledForm.timezone"
                    label="Time zone"
                    :options="timezoneOptions"
                  />
                </div>

                <InputElement
                  id="sc-pass"
                  v-model="scheduledForm.passcode"
                  label="Passcode"
                  :hide-label="false"
                  monospace
                />

                <div class="divide-y divide-stone-100 rounded-xl border border-stone-200">
                  <ToggleElement
                    v-model="scheduledForm.waitingRoom"
                    label="Waiting room"
                    description="Hold guests until you admit them."
                  />
                  <ToggleElement
                    v-model="scheduledForm.requestToJoin"
                    label="Request to join"
                    description="Ask for your approval before anyone enters."
                  />
                </div>
              </form>
            </div>

            <div
              class="flex flex-wrap items-center justify-end gap-x-2 gap-y-2 border-t border-stone-100 px-5 py-4 sm:px-6"
            >
              <p v-if="submitError" role="alert" class="mr-auto text-sm text-danger">
                {{ submitError }}
              </p>
              <button
                type="button"
                @click="closeNewMeeting"
                class="rounded-xl border border-stone-200 px-4 py-2.5 text-sm font-medium text-gray-700 transition hover:bg-stone-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
              >
                Cancel
              </button>
              <button
                type="button"
                @click="submitMeeting"
                :disabled="submitting"
                :aria-busy="submitting"
                class="rounded-xl bg-primary px-5 py-2.5 text-sm font-medium text-white transition hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary disabled:opacity-60"
              >
                {{ submitting ? 'Working…' : mode === 'now' ? 'Start meeting' : 'Schedule' }}
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>
