<script setup lang="ts">
import { reactive, ref } from 'vue'
import { nanoid } from 'nanoid'
import { PopUpWidth, type NewMeetingMode } from '@/types/video-call'
import PopUpComponent from '@/components/ui/PopUpComponent.vue'
import { WebcamIcon } from 'lucide-vue-next'
import InputElement from '../ui/InputElement.vue'
import ToggleElement from '../ui/ToggleElement.vue'
import SelectElement from '../ui/SelectElement.vue'
import { useAuthStore } from '@/stores/auth.ts'
import { api } from '@/lib/axios.ts'
import { useRouter } from 'vue-router'

const router = useRouter()

const props = defineProps<{
  is_open: boolean
  mode: NewMeetingMode
}>()

const emits = defineEmits<{
  close: []
  switchNewMeetingMode: [NewMeetingMode]
}>()

const modes: { key: NewMeetingMode; label: string }[] = [
  { key: 'now', label: 'Start now' },
  { key: 'scheduled', label: 'Schedule' },
]

const today = () => new Date().toLocaleDateString('en-CA')
const genPasscode = () => Math.random().toString(36).slice(2, 8)
const personalId = nanoid()

const scheduledForm = reactive({
  topic: '',
  date: today(),
  time: '09:00',
  duration: '45 minutes',
  timezone: 'WIB (UTC+7)',
  passcode: genPasscode(),
  waitingRoom: true,
})

const nowForm = reactive({
  topic: "Arfan's meeting",
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

const currentUserId = (): string => {
  const token = useAuthStore().accessToken
  if (!token) return ''
  try {
    const base64 = (token.split('.')[1] ?? '').replace(/-/g, '+').replace(/_/g, '/')
    const payload = JSON.parse(atob(base64))
    return String(payload.sub ?? '')
  } catch {
    return ''
  }
}

const resolveAlias = () => {
  if (nowForm.meetingId === 'personal') return personalId
  if (nowForm.meetingId === 'custom') return nowForm.customId.trim() || nanoid()
  return nanoid()
}

const submitMeeting = async () => {
  if (submitting.value || props.mode !== 'now') return
  submitError.value = ''
  submitting.value = true

  try {
    const alias = resolveAlias()
    await api.post('/room', {
      title: nowForm.topic.trim() || 'Untitled meeting',
      alias,
      owner_id: currentUserId(),
      passcode: genPasscode(),
      options: JSON.stringify({
        waiting_room: nowForm.waitingRoom,
      }),
    })

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

const radioCls =
  'flex cursor-pointer items-start gap-x-3 rounded-xl border border-stone-200 p-3.5 transition has-checked:border-primary has-checked:bg-primary/5 hover:border-stone-300'
</script>

<template>
  <PopUpComponent :is-open="is_open" @close="emits('close')" :width="PopUpWidth.MEDIUM">
    <template #title>
      <div class="flex items-start gap-x-3">
        <span class="grid size-10 shrink-0 place-items-center rounded-xl bg-primary/10 text-primary"
          aria-hidden="true">
          <WebcamIcon class="size-5" />
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
    </template>

    <div class="px-5 sm:px-6">
      <div role="tablist" class="grid grid-cols-2 gap-1 rounded-xl bg-stone-100 p-1 my-2">
        <button v-for="m in modes" :key="m.key" role="tab" type="button" :aria-selected="mode === m.key"
          @click="emits('switchNewMeetingMode', m.key)"
          class="rounded-lg px-4 py-2 text-sm font-medium transition focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-primary"
          :class="props.mode === m.key
            ? 'bg-white text-gray-900 shadow-sm'
            : 'text-gray-500 hover:text-gray-800'">
          {{ m.label }}
        </button>
      </div>
    </div>

    <div class="flex-1 overflow-y-auto px-5 py-5 sm:px-6">
      <form v-if="mode === 'now'" class="flex flex-col gap-y-5" @submit.prevent="submitMeeting">
        <InputElement id="now-topic" v-model="nowForm.topic" label="Topic" :hide-label="false"
          placeholder="Team standup" />

        <fieldset class="flex flex-col gap-y-2">
          <legend class="mb-1 text-xs font-semibold tracking-wide text-gray-500 uppercase">
            Meeting ID
          </legend>

          <label :class="radioCls">
            <input v-model="nowForm.meetingId" type="radio" value="auto" class="mt-0.5 size-4 accent-primary" />
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
            <input v-model="nowForm.meetingId" type="radio" value="personal" class="mt-0.5 size-4 accent-primary" />
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
            <input v-model="nowForm.meetingId" type="radio" value="custom" class="mt-0.5 size-4 accent-primary" />
            <span class="min-w-0">
              <span class="block text-sm font-medium text-gray-800">Custom</span>
              <span class="mt-0.5 block text-xs text-gray-500">
                Choose an ID yourself.
              </span>
            </span>
          </label>

          <InputElement v-if="nowForm.meetingId === 'custom'" id="now-custom-id" v-model="nowForm.customId"
            label="Custom ID" :hide-label="false" placeholder="my-room" monospace />
        </fieldset>

        <div class="divide-y divide-stone-100 rounded-xl border border-stone-200">
          <ToggleElement v-for="t in nowToggles" :key="t.key" v-model="nowForm[t.key]" :label="t.label"
            :description="t.description" />
        </div>
      </form>

      <form v-else class="flex flex-col gap-y-5" @submit.prevent="submitMeeting">
        <InputElement id="sc-topic" v-model="scheduledForm.topic" label="Topic" :hide-label="false"
          placeholder="Weekly sync" />

        <div class="grid gap-4 sm:grid-cols-2">
          <InputElement id="sc-date" v-model="scheduledForm.date" type="date" label="Date" :hide-label="false"
            :min="today()" />
          <InputElement id="sc-time" v-model="scheduledForm.time" type="time" label="Start time"
            :hide-label="false" />
        </div>

        <div class="grid gap-4 sm:grid-cols-2">
          <SelectElement id="sc-dur" v-model="scheduledForm.duration" label="Duration" :options="durationOptions" />
          <SelectElement id="sc-tz" v-model="scheduledForm.timezone" label="Time zone" :options="timezoneOptions" />
        </div>

        <InputElement id="sc-pass" v-model="scheduledForm.passcode" label="Passcode" :hide-label="false" monospace />

        <div class="divide-y divide-stone-100 rounded-xl border border-stone-200">
          <ToggleElement v-model="scheduledForm.waitingRoom" label="Waiting room"
            description="Hold guests until you admit them." />
        </div>
      </form>
    </div>

    <div class="flex flex-wrap items-center justify-end gap-x-2 gap-y-2 border-t border-stone-100 px-5 py-4 sm:px-6">
      <p v-if="submitError" role="alert" class="mr-auto text-sm text-danger">
        {{ submitError }}
      </p>
      <button type="button" @click="emits('close')"
        class="rounded-xl border border-stone-200 px-4 py-2.5 text-sm font-medium text-gray-700 transition hover:bg-stone-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary cursor-pointer">
        Cancel
      </button>
      <button type="button" @click="submitMeeting" :disabled="submitting" :aria-busy="submitting"
        class="rounded-xl bg-primary px-5 py-2.5 text-sm font-medium text-white transition hover:bg-primary-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary disabled:opacity-60 cursor-pointer">
        {{ submitting ? 'Working…' : mode === 'now' ? 'Start meeting' : 'Schedule' }}
      </button>
    </div>
  </PopUpComponent>
</template>
