<script setup lang="ts">
import { computed, watch } from 'vue'
import { useRoute } from 'vue-router'
import CallControls from '@/components/video-call/CallControls.vue'
import CallHeader from '@/components/video-call/CallHeader.vue'
import CallStatusScreen from '@/components/video-call/CallStatusScreen.vue'
import FloatingReactions from '@/components/video-call/FloatingReactions.vue'
import JoinRequestToast from '@/components/video-call/JoinRequestToast.vue'
import VideoGrid from '@/components/video-call/VideoGrid.vue'
import { useVideoCallRoom } from '@/composables/useVideoCallRoom'
import WelcomeToast from '@/components/video-call/membersChangesToast.vue'
import NewMessageToast from '@/components/video-call/NewMessageToast.vue'
import MembersPopup from '@/components/video-call/MembersPopup.vue'
import { provideMembersPanel } from '@/composables/useMembersPanel.ts'
import { provideConfigPanel } from '@/composables/useConfigPanel.tsx'
import ConfigPopup from '@/components/video-call/ConfigPopup.vue'
import ChatWindow from '@/components/video-call/ChatWindow.vue'
import { useCallStore } from '@/stores/call.ts'
import { useAuthStore } from '@/stores/auth'
import { recordRecentMeeting } from '@/lib/recentMeetings'
import { userIdFromToken } from '@/lib/token'
import { storeToRefs } from 'pinia'

const route = useRoute()

const {
  participants,
  room,
  cameraOff,
  floating,
  currentRequest,
  remainingRequests,
  showReaction,
  resolveRequest,
  toggleCamera,
  toggleMic,
  muted,
  hangup,
  memberChangesData,
  resolveNewMembers,
  newMessageData,
  resolveNewMessage,
  sendControl,
  sendChat,
} = useVideoCallRoom(() => route.params.id)

const membersPanel = provideMembersPanel(participants)
const configPanel = provideConfigPanel()

const roomParamId = computed(() => String(route.params.id))

const auth = useAuthStore()

watch(
  () => room.value.status,
  (status) => {
    if (status !== 'joined') return
    const ownerId = userIdFromToken(auth.accessToken)
    if (!ownerId) return
    recordRecentMeeting(ownerId, {
      alias: roomParamId.value,
      title: room.value.title,
    })
  },
)

const roomStatus = computed<'denied' | 'waiting' | 'not-started' | 'not-found'>(() => {
  if (room.value.status === 'denied') return 'denied'
  if (room.value.status === 'waiting') return 'waiting'
  if (room.value.status === 'not-started') return 'not-started'
  return 'not-found'
})

const store = useCallStore()
const { isChatOpen } = storeToRefs(store)
</script>

<template>
  <div
    v-if="room.status === 'loading'"
    class="relative flex flex-col w-full h-dvh bg-gray-100 items-center justify-center overflow-hidden"
  >
    Please Wait
  </div>

  <div
    v-else-if="room.status === 'joined'"
    class="flex flex-col w-full h-dvh bg-gray-100 overflow-hidden"
  >
    <CallHeader :roomTitle="room.title" class="shrink-0" />
    <div class="flex flex-1 min-h-0">
      <VideoGrid :participants="participants" class="flex-1 min-w-0" />
      <ChatWindow v-if="isChatOpen" @send="sendChat" />
    </div>
    <FloatingReactions :reactions="floating" />

    <JoinRequestToast
      :request="currentRequest"
      :remaining="remainingRequests"
      @admit="resolveRequest('admit')"
      @deny="resolveRequest('deny')"
    />

    <WelcomeToast :user="memberChangesData" @resolve="resolveNewMembers" />
    <NewMessageToast :message="newMessageData" @resolve="resolveNewMessage" />
    <members-popup v-if="membersPanel.isOpen" />
    <ConfigPopup v-if="configPanel.isOpen" />

    <CallControls
      :camera-off="cameraOff"
      :muted="muted"
      @toggle-camera="toggleCamera"
      @toggle-mic="toggleMic"
      @hangup="hangup"
      @select-reaction="showReaction"
      class="shrink-0"
      @send-control="sendControl"
    />
  </div>

  <!-- room exists, user is not in the room yet / room not found -->
  <CallStatusScreen v-else :status="roomStatus" :room-id="roomParamId" :room-owner="room.owner" />
</template>
