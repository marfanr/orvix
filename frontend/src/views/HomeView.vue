<script setup lang="ts">
import {
  ArrowRightIcon,
  BellIcon,
  CalendarDaysIcon,
  ClockIcon,
  LogOutIcon,
  PlusIcon,
  SettingsIcon,
  SparklesIcon,
  UserIcon,
  UsersIcon,
  VideoIcon,
} from 'lucide-vue-next'
import { onMounted, onUnmounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import HeaderBrand from '@/components/HeaderBrand.vue'
import NewMeetingPopup from '@/components/home/NewMeetingPopup.vue'
import { useAuthStore } from '@/stores/auth'
import type { NewMeetingMode } from '@/types/video-call'

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
const mode = ref<NewMeetingMode>('now')
const router = useRouter()
const auth = useAuthStore()

const handleLogout = async () => {
  closeProfileMenu()
  await auth.logout()
  await router.push('/signin')
}

const todayLabel = new Date().toLocaleDateString('en-US', {
  weekday: 'long',
  month: 'long',
  day: 'numeric',
})

const upcomingMeetings = ref([
  { id: 1, title: 'Team Standup', time: '09:00', date: 'Today', participants: 8 },

])

const recentMeetings = ref<{
  id: number,
  title: string,
  time: string,
  date: string,
  participants: number
}[]>([
  // { id: 4, title: 'Sprint Planning', time: '10:00', date: 'Yesterday', participants: 10 },
])

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

const onKeydown = (e: KeyboardEvent) => {
  if (e.key !== 'Escape') return
  else if (showProfileMenu.value) closeProfileMenu()
}
onMounted(() => document.addEventListener('keydown', onKeydown))
onUnmounted(() => {
  document.removeEventListener('keydown', onKeydown)
})

/* shared classes */
const menuItemCls =
  'flex w-full items-center gap-x-2.5 rounded-lg px-3 py-2 text-left text-sm text-gray-700 transition hover:bg-stone-100 focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-primary'
</script>

<template>
  <div class="min-h-screen bg-primary-soft/30" @click="closeProfileMenu">
    <header class="sticky top-0 z-40 border-b border-stone-200/80 bg-white/80 backdrop-blur-md">
      <div class="mx-auto flex h-16 max-w-6xl items-center gap-2 px-4 sm:gap-3 sm:px-6 lg:px-8">
        <div class="flex items-center gap-x-2.5">
          <HeaderBrand />
        </div>

        <nav class="ml-1 flex items-center gap-1 rounded-full bg-stone-100/80 p-1 sm:ml-3" aria-label="Primary">
          <button v-for="item in navigationItems" :key="item.name" type="button" @click="setActive(item.name)"
            :aria-current="item.active ? 'page' : undefined" :class="[
              'rounded-full px-3 py-1.5 text-xs font-medium transition-all sm:px-4 sm:text-sm',
              item.active
                ? 'bg-white text-gray-900 shadow-sm'
                : 'text-gray-500 hover:text-gray-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary',
            ]">
            {{ item.name }}
          </button>
        </nav>

        <div class="ml-auto flex items-center gap-1 sm:gap-2">
          <button type="button" aria-label="Notifications"
            class="relative grid size-10 place-items-center rounded-full text-gray-600 transition hover:bg-stone-100 hover:text-gray-900 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary">
            <BellIcon class="size-5" aria-hidden="true" />
            <span class="absolute top-2.5 right-2.5 size-2 rounded-full bg-danger ring-2 ring-white"
              aria-hidden="true"></span>
          </button>

          <div class="relative" @click.stop>
            <button type="button" @click="toggleProfileMenu" aria-haspopup="true" :aria-expanded="showProfileMenu"
              aria-label="Account menu"
              class="grid size-9 place-items-center rounded-full bg-primary text-sm font-semibold text-white transition hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary">
              <span aria-hidden="true">A</span>
            </button>

            <div v-if="showProfileMenu"
              class="absolute right-0 z-50 mt-2 w-56 overflow-hidden rounded-xl border border-stone-200 bg-white p-1.5 shadow-xl">
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
              <button type="button" :class="[menuItemCls, 'text-danger hover:bg-danger/5']" @click="handleLogout">
                <LogOutIcon class="size-4" aria-hidden="true" />
                Logout
              </button>
            </div>
          </div>
        </div>
      </div>
    </header>

    <main id="main" class="mx-auto max-w-6xl px-4 py-6 sm:px-6 sm:py-10 lg:px-8">
      <!-- <div v-if="submitSuccess" role="status"
        class="mb-6 flex items-start gap-x-3 rounded-xl border border-success/20 bg-white px-4 py-3 text-sm text-gray-700 shadow-sm">
        <CheckCircle2Icon class="mt-0.5 size-5 shrink-0 text-success" aria-hidden="true" />
        <p class="min-w-0 flex-1">{{ submitSuccess }}</p>
        <button type="button" aria-label="Dismiss message"
          class="grid size-6 shrink-0 place-items-center rounded-full text-gray-400 transition hover:bg-stone-100 hover:text-gray-700 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
          @click="submitSuccess = ''">
          <XIcon class="size-4" aria-hidden="true" />
        </button>
      </div> -->

      <section
        class="relative overflow-hidden rounded-3xl bg-primary px-5 py-8 text-white shadow-xl shadow-primary/20 sm:px-8 sm:py-10 lg:px-10">
        <div class="pointer-events-none absolute -top-24 -right-16 size-64 rounded-full bg-white/10 blur-2xl"
          aria-hidden="true"></div>
        <div class="pointer-events-none absolute -bottom-28 left-1/3 size-56 rounded-full bg-primary-hover/30 blur-2xl"
          aria-hidden="true"></div>

        <div class="relative flex flex-col gap-8 lg:flex-row lg:items-end lg:justify-between">
          <div class="max-w-xl">
            <p class="flex items-center gap-x-2 text-xs font-medium tracking-wide text-white/70 uppercase">
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
              <button type="button"
                class="inline-flex items-center gap-x-2 rounded-xl bg-white px-4 py-2.5 text-sm font-semibold text-primary shadow-sm transition hover:bg-white/90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
                @click="openNewMeeting">
                <PlusIcon class="size-4" aria-hidden="true" />
                New meeting
              </button>
              <button type="button"
                class="inline-flex items-center gap-x-2 rounded-xl bg-white/10 px-4 py-2.5 text-sm font-medium text-white ring-1 ring-white/25 transition hover:bg-white/20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
                @click="openScheduledMeeting">
                <CalendarDaysIcon class="size-4" aria-hidden="true" />
                Schedule
              </button>
            </div>
          </div>

          <div class="hidden w-64 shrink-0 rounded-2xl bg-white/10 p-4 ring-1 ring-white/20 backdrop-blur-sm lg:block">
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
              <span class="rounded-full bg-white px-2.5 py-1 text-xs font-medium text-gray-500 ring-1 ring-stone-200">
                {{ upcomingMeetings.length }} scheduled
              </span>
            </div>

            <div v-if="upcomingMeetings.length" class="mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
              <article v-for="meeting in upcomingMeetings" :key="meeting.id"
                class="group flex flex-col rounded-2xl border border-stone-200 bg-white p-4 shadow-sm transition hover:-translate-y-0.5 hover:border-primary/30 hover:shadow-md">
                <div class="flex items-center justify-between gap-2">
                  <span
                    class="inline-flex items-center gap-x-1.5 rounded-full bg-primary/10 px-2.5 py-1 text-[11px] font-semibold text-primary">
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
                  class="mt-4 inline-flex items-center gap-x-1 text-xs font-medium text-primary opacity-0 transition group-hover:opacity-100">
                  View details
                  <ArrowRightIcon class="size-3.5" aria-hidden="true" />
                </span>
              </article>
            </div>

            <div v-else
              class="mt-4 flex flex-col items-center justify-center rounded-2xl border border-dashed border-stone-300 bg-white/60 px-6 py-10 text-center">
              <span class="grid size-11 place-items-center rounded-full bg-primary/10 text-primary">
                <CalendarDaysIcon class="size-5" aria-hidden="true" />
              </span>
              <p class="mt-3 text-sm font-medium text-gray-800">No meetings scheduled</p>
              <p class="mt-1 text-sm text-gray-500">Plan one and it will show up here.</p>
              <button type="button"
                class="mt-4 inline-flex items-center gap-x-2 rounded-xl bg-primary px-4 py-2.5 text-sm font-medium text-white transition hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
                @click="openScheduledMeeting">
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
              class="mt-4 divide-y divide-stone-100 overflow-hidden rounded-2xl border border-stone-200 bg-white shadow-sm">
              <li v-for="meeting in recentMeetings" :key="meeting.id">
                <div class="flex items-center gap-x-3 px-4 py-3.5 transition hover:bg-stone-50 sm:px-5">
                  <span class="grid size-11 shrink-0 place-items-center rounded-xl bg-primary/10 text-primary"
                    aria-hidden="true">
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
              class="mt-2 overflow-hidden rounded-lg bg-stone-50 px-3 py-2 font-mono text-xs text-gray-700 break-all ring-1 ring-stone-100">
              <!-- {{ personalId }} -->
              ss
            </p>

            <button type="button"
              class="mt-4 inline-flex w-full items-center justify-center gap-x-2 rounded-xl bg-primary px-4 py-2.5 text-sm font-medium text-white transition hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
              @click="openNewMeeting">
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
            <button type="button"
              class="mt-4 inline-flex w-full items-center justify-center gap-x-2 rounded-xl border border-stone-200 bg-white px-4 py-2.5 text-sm font-medium text-gray-700 transition hover:border-primary/40 hover:text-primary focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary"
              @click="openScheduledMeeting">
              <CalendarDaysIcon class="size-4" aria-hidden="true" />
              Schedule a meeting
            </button>
          </div>
        </aside>
      </div>
    </main>

    <NewMeetingPopup @close="closeNewMeeting" :is_open="showNewMeeting" :mode="mode"
      @switch-new-meeting-mode="switchNewMeetingMode" />
  </div>
</template>
