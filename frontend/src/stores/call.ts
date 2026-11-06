import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type {
  ChatMessage,
  FloatingReaction,
  JoinRequest,
  MemberChanges,
  MemberChangesType,
  NewMessageEvent,
  Participant,
  RoomStatus,
} from '@/types/video-call'

export interface RoomState {
  id: string | undefined
  owner: string | undefined
  status: RoomStatus
  title: string
}

export const useCallStore = defineStore('call', () => {
  // Room State
  const room = ref<RoomState>({
    id: undefined,
    owner: undefined,
    status: 'loading',
    title: 'Untitled room',
  })

  const localRawStream = ref<MediaStream | null>(null)

  const participants = ref<Participant[]>([
    {
      id: 0,
      conn_id: undefined,
      name: 'me',
      muted: true,
      cameraOff: true,
      stream: null,
    },
  ])

  //  Chat State
  let msgSeq = 0
  const chatMessages = ref<ChatMessage[]>([])

  function pushChat(msg: string, sender: string, outgoing: boolean, timestamp: number) {
    chatMessages.value.push({
      id: msgSeq++,
      outgoing,
      sender,
      text: msg,
      time: new Date(timestamp).toLocaleTimeString('id-ID', {
        hour: '2-digit',
        minute: '2-digit',
      }),
    })
  }

  const isChatOpen = ref<boolean>(false)
  const toggleChat = () => {
    isChatOpen.value = !isChatOpen.value
  }
  const closeChat = () => {
    isChatOpen.value = false
  }

  //  Toast Queues
  let welcomeSeq = 0
  const memberChanges = ref<MemberChanges[]>([])
  const memberChangesData = computed(() => memberChanges.value[0])

  function addMemberChange(username: string, type: MemberChangesType) {
    memberChanges.value.push({
      eventId: ++welcomeSeq,
      username,
      type,
    })
  }

  function resolveMemberChange() {
    memberChanges.value.shift()
  }

  let newMsgSeq = 0
  const newMessages = ref<NewMessageEvent[]>([])
  const newMessageData = computed(() => newMessages.value[0])

  function addNewMessage(text: string, sender: string) {
    newMessages.value.push({
      eventId: ++newMsgSeq,
      sender,
      text,
    })
  }

  function resolveNewMessage() {
    newMessages.value.shift()
  }

  const joinRequests = ref<JoinRequest[]>([])
  const currentRequest = computed(() => joinRequests.value[0])
  const remainingRequests = computed(() => joinRequests.value.length - 1)

  function addJoinRequest(request: JoinRequest) {
    if (!joinRequests.value.some((r) => r.conn_id === request.conn_id)) {
      joinRequests.value.push(request)
    }
  }

  function cancelJoinRequest(connId: string) {
    joinRequests.value = joinRequests.value.filter((r) => r.conn_id !== connId)
  }

  function shiftJoinRequest() {
    joinRequests.value.shift()
  }

  //  Reactions
  let reactionSeq = 0
  const floating = ref<FloatingReaction[]>([])

  function showReaction(emoji: string) {
    const key = ++reactionSeq
    floating.value.push({ key, emoji, left: 10 + Math.random() * 80 })
    setTimeout(() => {
      floating.value = floating.value.filter((r) => r.key !== key)
    }, 2500)
  }

  //  Participant Actions
  function upsertParticipant(p: Participant) {
    const existing = participants.value.find((item) => item.conn_id === p.conn_id)
    if (existing) {
      existing.name = p.name
    } else {
      participants.value.push(p)
    }
  }

  function removeParticipant(connId: string) {
    const idx = participants.value.findIndex((item) => item.conn_id === connId)
    if (idx !== -1) participants.value.splice(idx, 1)
  }

  return {
    room,
    participants,
    chatMessages,
    pushChat,
    memberChanges,
    memberChangesData,
    addMemberChange,
    resolveMemberChange,
    newMessages,
    newMessageData,
    addNewMessage,
    resolveNewMessage,
    joinRequests,
    currentRequest,
    remainingRequests,
    addJoinRequest,
    cancelJoinRequest,
    shiftJoinRequest,
    floating,
    showReaction,
    upsertParticipant,
    removeParticipant,
    isChatOpen,
    toggleChat,
    closeChat,
    localRawStream,
  }
})
