<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { nanoid } from 'nanoid'
import { PopUpWidth, type NewMeetingMode } from '@/types/video-call'
import PopUpComponent from '@/components/ui/PopUpComponent.vue'
import { WebcamIcon } from 'lucide-vue-next'
import InputElement from '../ui/InputElement.vue'
import ToggleElement from '../ui/ToggleElement.vue'
import SelectElement from '../ui/SelectElement.vue'
import { useAuthStore } from '@/stores/auth.ts'
import { api } from '@/lib/axios.ts'
import { userIdFromToken } from '@/lib/token'
import { saveScheduledMeeting, type ScheduledMeeting } from '@/lib/scheduledMeetings'
import { useRouter } from 'vue-router'

const router = useRouter()

const props = defineProps<{
  is_open: boolean
  mode: NewMeetingMode
}>()

const emits = defineEmits<{
  close: []
  switchNewMeetingMode: [NewMeetingMode]
  scheduled: [ScheduledMeeting]
}>()

const modes: { key: NewMeetingMode; label: string }[] = [
  { key: 'now', label: 'Start now' },
  { key: 'scheduled', label: 'Schedule' },
]

const auth = useAuthStore()
const userId = computed(() => userIdFromToken(auth.accessToken))
const personalId = computed(() => (userId.value ? `me-${userId.value}` : ''))

const pad = (value: number) => String(value).padStart(2, '0')
const localDate = (date: Date) =>
  `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
const nextSlot = () => {
  const date = new Date()
  date.setHours(date.getHours() + 1, 0, 0, 0)
  return { date: localDate(date), time: `${pad(date.getHours())}:00` }
}

const today = () => localDate(new Date())
const initialSlot = nextSlot()

const scheduledForm = reactive({
  topic: '',
  date: initialSlot.date,
  time: initialSlot.time,
  duration: '45 minutes',
  timezone: 'WIB (UTC+7)',
  waitingRoom: true,
})

const nowForm = reactive({
  topic: '',
  meetingId: 'auto' as 'auto' | 'personal' | 'custom',
  customId: '',
  waitingRoom: true,
})

const nowToggles = [
  {
    key: 'waitingRoom',
    label: 'Waiting room',
    description: 'Hold guests until the host admits them.',
  },
] as const

const durations = ['15 minutes', '30 minutes', '45 minutes', '1 hour', '2 hours']
const timezones = ['WIB (UTC+7)', 'WITA (UTC+8)', 'WIT (UTC+9)']

const durationOptions = durations.map((duration) => ({ value: duration, label: duration }))
const timezoneOptions = timezones.map((timezone) => ({ value: timezone, label: timezone }))

const submitting = ref(false)
const submitError = ref('')

const resolveAlias = () => {
  if (nowForm.meetingId === 'personal') return personalId.value || nanoid()
  if (nowForm.meetingId === 'custom') return nowForm.customId.trim() || nanoid()
  return nanoid()
}

const parseDurationMinutes = (value: string) => {
  const amount = Number.parseInt(value, 10)
  if (!Number.isFinite(amount)) return 45
  return value.includes('hour') ? amount * 60 : amount
}

const scheduledStart = () => {
  const dateMatch = /^(\d{4})-(\d{2})-(\d{2})$/.exec(scheduledForm.date)
  const timeMatch = /^(\d{2}):(\d{2})$/.exec(scheduledForm.time)
  if (!dateMatch || !timeMatch) return null

  const offset = /UTC([+-]\d+)/.exec(scheduledForm.timezone)
  const offsetHours = offset ? Number(offset[1]) : 0
  const start = new Date(
    Date.UTC(
      Number(dateMatch[1]),
      Number(dateMatch[2]) - 1,
      Number(dateMatch[3]),
      Number(timeMatch[1]) - offsetHours,
      Number(timeMatch[2]),
    ),
  )

  return Number.isNaN(start.getTime()) ? null : start
}

const submitMeeting = async () => {
  if (submitting.value) return
  submitError.value = ''

  const ownerId = userId.value
  if (!ownerId) {
    submitError.value = 'Your session expired. Sign in again and retry.'
    return
  }

  submitting.value = true

  try {
    if (props.mode === 'scheduled') {
      const startAt = scheduledStart()
      if (!startAt) {
        submitError.value = 'Choose a valid date and time.'
        return
      }
      if (startAt.getTime() <= Date.now()) {
        submitError.value = 'Choose a date and time in the future.'
        return
      }

      const alias = nanoid()
      const title = scheduledForm.topic.trim() || 'Untitled meeting'
      const durationMin = parseDurationMinutes(scheduledForm.duration)

      await api.post('/room', {
        title,
        alias,
        owner_id: ownerId,
        options: JSON.stringify({
          waiting_room: scheduledForm.waitingRoom,
        }),
      })

      const meeting: ScheduledMeeting = {
        id: alias,
        alias,
        title,
        startAt: startAt.toISOString(),
        durationMin,
        timezone: scheduledForm.timezone,
      }

      saveScheduledMeeting(ownerId, meeting)
      emits('scheduled', meeting)
      emits('close')
      return
    }

    const alias = resolveAlias()
    try {
      await api.post('/room', {
        title: nowForm.topic.trim() || 'Untitled meeting',
        alias,
        owner_id: ownerId,
        options: JSON.stringify({
          waiting_room: nowForm.waitingRoom,
        }),
      })
    } catch (err) {
      const status = (err as { response?: { status?: number } }).response?.status
      if (!(nowForm.meetingId === 'personal' && status === 409)) throw err
    }

    emits('close')
    router.push(`/meet/${alias}`)
  } catch (err) {
    const axiosErr = err as { response?: { data?: { error?: string } } }
    submitError.value =
      axiosErr.response?.data?.error || 'Could not create the meeting. Please try again.'
  } finally {
    submitting.value = false
  }
}

watch(
  () => props.mode,
  () => {
    submitError.value = ''
  },
)

const radioCls =
  'flex cursor-pointer items-start gap-x-3 rounded-xl border border-stone-200 p-3.5 transition has-checked:border-primary has-checked:bg-primary/5 hover:border-stone-300'
</script>

<template>
  <PopUpComponent :is-open="is_open" @close="emits('close')" :width="PopUpWidth.MEDIUM">
    <template #title>
      <div class="flex items-start gap-x-3">
        <span
          class="grid size-10 shrink-0 place-items-center rounded-xl bg-primary/10 text-primary"
          aria-hidden="true"
        >
          <WebcamIcon class="size-5" />
        </span>
        <div>
          <h3 id="nm-title" class="font-heading text-lg font-semibold text-gray-900">
            New meeting
          </h3>
          <p class="mt-0.5 text-xs text-gray-500">Start instantly or schedule it for later.</p>
        </div>
      </div>
    </template>

    <div class="px-5 sm:px-6">
      <div role="tablist" class="grid grid-cols-2 gap-1 rounded-xl bg-stone-100 p-1 my-2">
        <button
          v-for="m in modes"
          :key="m.key"
          role="tab"
          type="button"
          :aria-selected="mode === m.key"
          @click="emits('switchNewMeetingMode', m.key)"
          class="rounded-lg px-4 py-2 text-sm font-medium transition focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-primary"
          :class="
            props.mode === m.key
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
        id="now-form"
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
              <span class="block text-sm font-medium text-gray-800"> Generate automatically </span>
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
              <span class="block text-sm font-medium text-gray-800"> Personal meeting ID </span>
              <span class="mt-0.5 block text-xs text-gray-500">
                Reuse the same room for every call.
              </span>
            </span>
            <span
              class="max-w-40 shrink-0 truncate font-mono text-[11px] text-gray-500"
              :title="personalId"
            >
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
              <span class="mt-0.5 block text-xs text-gray-500"> Choose an ID yourself. </span>
            </span>
          </label>

          <InputElement
            v-if="nowForm.meetingId === 'custom'"
            id="now-custom-id"
            v-model="nowForm.customId"
            label="Custom ID"
            :hide-label="false"
            placeholder="my-room"
            monospace
          />
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

      <form
        v-else
        id="scheduled-form"
        class="flex flex-col gap-y-5"
        @submit.prevent="submitMeeting"
      >
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

        <div class="divide-y divide-stone-100 rounded-xl border border-stone-200">
          <ToggleElement
            v-model="scheduledForm.waitingRoom"
            label="Waiting room"
            description="Hold guests until you admit them."
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
        @click="emits('close')"
        class="rounded-xl border border-stone-200 px-4 py-2.5 text-sm font-medium text-gray-700 transition hover:bg-stone-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary cursor-pointer"
      >
        Cancel
      </button>
      <button
        type="submit"
        :form="mode === 'now' ? 'now-form' : 'scheduled-form'"
        :disabled="submitting"
        :aria-busy="submitting"
        class="rounded-xl bg-primary px-5 py-2.5 text-sm font-medium text-white transition hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary disabled:opacity-60 cursor-pointer"
      >
        {{ submitting ? 'Working…' : mode === 'now' ? 'Start meeting' : 'Schedule' }}
      </button>
    </div>
  </PopUpComponent>
</template>
